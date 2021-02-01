use std::{collections::HashMap, str::FromStr, sync::Arc};

use chrono::{Duration, NaiveDateTime, Utc};
use mongodb::{
    bson::{self, doc, Document},
    error::Result as MongoResult,
    Database,
};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateMessage},
    http::client::Http,
    prelude::RwLock,
};

use crate::{
    launch_tracking,
    models::{
        launches::{LaunchData, LaunchStatus},
        reminders::Reminder,
    },
    utils::{
        constants::{DEFAULT_COLOR, DEFAULT_ICON, LAUNCH_AGENCIES},
        format_duration,
        reminders::{get_guild_settings, get_user_settings},
    },
};

pub async fn reminder_tracking(http: Arc<Http>, cache: Arc<RwLock<Vec<LaunchData>>>, db: Database) {
    // wait for client to have started
    tokio::time::delay_for(std::time::Duration::from_secs(60)).await;

    let mut loop_count: i64 = 0;
    let mut reminded: HashMap<String, i64> = HashMap::new();

    loop {
        println!("running loop {}", loop_count);

        if loop_count % 5 == 0 {
            tokio::spawn(launch_tracking(http.clone(), db.clone(), cache.clone()));
        }

        loop_count += 1;

        let launches: Vec<LaunchData> = cache
            .read()
            .await
            .iter()
            .filter(|l| l.status == LaunchStatus::Go)
            .cloned()
            .collect();
        if launches.is_empty() {
            tokio::time::delay_for(std::time::Duration::from_secs(55)).await;
            continue;
        }

        let now = Utc::now().timestamp();

        for l in launches {
            let difference = l.net - NaiveDateTime::from_timestamp(now, 0);

            if let Some(dur) = reminded.get(&l.ll_id) {
                if *dur == difference.num_minutes() {
                    continue;
                }
            }
            reminded.insert(l.ll_id.clone(), difference.num_minutes());

            if let Ok(Some(r)) = get_reminders(&db, difference.num_minutes()).await {
                if let Ok(res) = bson::from_bson(r.into()) {
                    tokio::spawn(execute_reminder(db.clone(), http.clone(), res, l.clone(), difference));
                }
            }
        }

        tokio::time::delay_for(std::time::Duration::from_secs(55)).await;
    }
}

async fn get_reminders(db: &Database, minutes: i64) -> MongoResult<Option<Document>> {
    db.collection("reminders")
        .find_one(doc! { "minutes": minutes }, None)
        .await
}

async fn execute_reminder(
    db: Database,
    http: Arc<Http>,
    reminder: Reminder,
    l: LaunchData,
    difference: Duration,
) {
    'channel: for c in &reminder.channels {
        let settings_res = get_guild_settings(&db, c.guild.into()).await;

        if let Ok(settings) = &settings_res {
            for filter in &settings.filters {
                if let Some(agency) = LAUNCH_AGENCIES.get(filter.as_str()) {
                    if *agency == l.lsp {
                        continue 'channel;
                    }
                }
            }
        }

        let _ = c
            .channel
            .send_message(&http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| reminder_embed(e, &l, difference));

                if let Ok(settings) = &settings_res {
                    if !settings.mentions.is_empty() {
                        let mut mentions = String::new();
                        for mention in &settings.mentions {
                            mentions.push_str(&format!(" <@&{}>", mention.as_u64()))
                        }
                        m.content(mentions);
                    }
                }

                m
            })
            .await;
    }

    'user: for u in &reminder.users {
        let settings_res = get_user_settings(&db, u.0).await;

        if let Ok(settings) = settings_res {
            for filter in &settings.filters {
                if let Some(agency) = LAUNCH_AGENCIES.get(filter.as_str()) {
                    if *agency == l.lsp {
                        continue 'user;
                    }
                }
            }
        }

        if let Ok(chan) = u.create_dm_channel(&http).await {
            let _ = chan
                .send_message(&http, |m: &mut CreateMessage| {
                    m.embed(|e: &mut CreateEmbed| reminder_embed(e, &l, difference))
                })
                .await;
        }
    }
}

fn reminder_embed<'a>(
    e: &'a mut CreateEmbed,
    l: &LaunchData,
    diff: Duration,
) -> &'a mut CreateEmbed {
    let live = if let Some(link) = l.vid_urls.first() {
        format!("**Live at:** {}", format_url(&link.url))
    } else {
        "".to_owned()
    };

    e.color(DEFAULT_COLOR)
        .author(|a: &mut CreateEmbedAuthor| {
            a.name(format!("{} till launch", format_duration(diff, false)))
                .icon_url(DEFAULT_ICON)
        })
        .description(format!(
            "**Payload:** {}\n\
            **Vehicle:** {}\n\
            **NET:** {}\n\
            {}",
            &l.payload,
            &l.vehicle,
            l.net.format("%d %B, %Y; %H:%m:%S UTC").to_string(),
            live
        ))
        .timestamp(l.net.format("%+").to_string());

    if let Some(img) = &l.rocket_img {
        e.thumbnail(img);
    }

    e
}

fn format_url(rawlink: &str) -> String {
    if let Ok(link) = url::Url::from_str(rawlink) {
        if let Some(mut domain) = link.domain() {
            domain = domain.trim_start_matches("www.");
            return format!("[{}]({})\n", domain, rawlink);
        }
    };
    rawlink.to_owned()
}

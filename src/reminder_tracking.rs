use std::{
    collections::HashMap,
    str::FromStr,
    sync::Arc,
};

use chrono::{
    Duration,
    NaiveDateTime,
    Utc,
};
use futures::{
    future,
    stream::{
        self,
        FuturesUnordered,
        StreamExt,
    },
};
use mongodb::{
    bson::{
        self,
        doc,
        Document,
    },
    error::Result as MongoResult,
    Database,
};
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
        CreateMessage,
    },
    http::client::Http,
    model::channel::Message,
    prelude::RwLock,
};

use crate::{
    launch_tracking,
    models::{
        launches::{
            LaunchData,
            LaunchStatus,
        },
        reminders::Reminder,
    },
    utils::{
        constants::{
            DEFAULT_COLOR,
            DEFAULT_ICON,
            LAUNCH_AGENCIES,
        },
        error_log,
        format_duration,
        reminders::{
            get_guild_settings,
            get_user_settings,
        },
    },
};

pub async fn reminder_tracking(http: Arc<Http>, cache: Arc<RwLock<Vec<LaunchData>>>, db: Database) {
    // wait for client to have started
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

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
            tokio::time::sleep(std::time::Duration::from_secs(55)).await;
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
                    let handle = tokio::spawn(execute_reminder(
                        db.clone(),
                        http.clone(),
                        res,
                        l.clone(),
                        difference,
                    ));

                    if let Err(e) = handle.await {
                        error_log(
                            http.clone(),
                            &format!("A panic happened in reminders: ```{:?}```", e),
                        )
                        .await
                    }
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(55)).await;
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
    let Reminder {
        channels,
        users,
        ..
    } = reminder;

    stream::iter(channels.into_iter())
        .filter_map(|c| {
            let db = db.clone();
            async move {
                get_guild_settings(&db, c.guild.into())
                    .await
                    .ok()
                    .map(|s| (c, s))
            }
        })
        .filter(|(_, settings)| {
            future::ready(
                !settings
                    .filters
                    .iter()
                    .filter_map(|filter| LAUNCH_AGENCIES.get(filter.as_str()))
                    .any(|agency| *agency == l.lsp),
            )
        })
        .map(|(c, settings)| {
            let mentions = settings
                .mentions
                .iter()
                .fold(String::new(), |acc, mention| {
                    acc + &format!(" <@&{}>", mention.as_u64())
                });

            (c, mentions)
        })
        .map(|(c, mentions)| {
            c.channel.send_message(&http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| reminder_embed(e, &l, difference));

                if !mentions.is_empty() {
                    m.content(mentions);
                }

                m
            })
        })
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;

    stream::iter(users.into_iter())
        .filter_map(|u| {
            let db = db.clone();
            async move { get_user_settings(&db, u.0).await.ok().map(|s| (u, s)) }
        })
        .filter(|(_, settings)| {
            future::ready(
                !settings
                    .filters
                    .iter()
                    .filter_map(|filter| LAUNCH_AGENCIES.get(filter.as_str()))
                    .any(|agency| *agency == l.lsp),
            )
        })
        .filter_map(|(u, _)| {
            let http = http.clone();
            async move { u.create_dm_channel(&http).await.ok() }
        })
        .map(|c| {
            c.id.send_message(&http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| reminder_embed(e, &l, difference))
            })
        })
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;
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
            a.name(format!("{} till launch", &format_duration(diff, false)))
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
        .timestamp(l.net.format("%Y-%m-%dT%H:%M:%S").to_string());

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

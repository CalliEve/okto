use std::sync::Arc;

use chrono::{Duration, NaiveDateTime, Utc};
use mongodb::{
    bson::{self, doc, Document},
    error::Result as MongoResult,
    sync::Database,
};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateMessage},
    http::client::Http,
    prelude::RwLock,
};

use crate::{
    models::{
        launches::{LaunchData, LaunchStatus},
        reminders::Reminder,
    },
    utils::{
        constants::{DEFAULT_COLOR, DEFAULT_ICON, LAUNCH_AGENCIES},
        format_duration,
    },
};

pub fn reminder_tracking(http: Arc<Http>, cache: Arc<RwLock<Vec<LaunchData>>>, db: Database) {
    loop {
        let launches: Vec<LaunchData> = cache
            .read()
            .iter()
            .filter(|l| l.status == LaunchStatus::Go)
            .cloned()
            .collect();
        if launches.is_empty() {
            std::thread::sleep(std::time::Duration::from_secs(55));
            continue;
        }

        let now = Utc::now().timestamp();

        for l in launches {
            let difference = l.net - NaiveDateTime::from_timestamp(now, 0);

            let reminders: Reminder =
                if let Ok(Some(r)) = get_reminders(&db, difference.num_minutes()) {
                    if let Ok(res) = bson::from_bson(r.into()) {
                        res
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

            'channel: for c in &reminders.channels {
                for filter in &c.filters {
                    if let Some(agency) = LAUNCH_AGENCIES.get(filter.as_str()) {
                        if *agency == &l.lsp {
                            continue 'channel;
                        }
                    }
                }

                let _ = c.channel.send_message(&http, |m: &mut CreateMessage| {
                    m.embed(|e: &mut CreateEmbed| reminder_embed(e, &l, difference));

                    if !c.mentions.is_empty() {
                        let mut mentions = String::new();
                        for mention in &c.mentions {
                            mentions.push_str(&format!(" <@&{}>", mention.as_u64()))
                        }
                        m.content(mentions);
                    }

                    m
                });
            }

            'user: for u in &reminders.users {
                for filter in &u.filters {
                    if let Some(agency) = LAUNCH_AGENCIES.get(filter.as_str()) {
                        if *agency == &l.lsp {
                            continue 'user;
                        }
                    }
                }

                if let Ok(chan) = u.user.create_dm_channel(&http) {
                    let _ = chan.send_message(&http, |m: &mut CreateMessage| {
                        m.embed(|e: &mut CreateEmbed| reminder_embed(e, &l, difference))
                    });
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(55));
        continue;
    }
}

fn get_reminders(db: &Database, minutes: i64) -> MongoResult<Option<Document>> {
    db.collection("reminders")
        .find_one(doc! { "minutes": minutes }, None)
}

fn reminder_embed<'a>(
    e: &'a mut CreateEmbed,
    l: &LaunchData,
    diff: Duration,
) -> &'a mut CreateEmbed {
    e.color(DEFAULT_COLOR)
        .author(|a: &mut CreateEmbedAuthor| {
            a.name(format!("NET {}", format_duration(diff)))
                .icon_url(DEFAULT_ICON)
        })
        .description(format!(
            "**Payload:** {}
            **Vehicle:** {}",
            &l.payload, &l.vehicle
        ));

    if let Some(img) = &l.rocket_img {
        e.thumbnail(img);
    }

    e
}

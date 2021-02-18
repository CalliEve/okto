use std::sync::Arc;

use futures::{
    future,
    stream::{
        self,
        FuturesUnordered,
    },
    StreamExt,
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
use serde::de::DeserializeOwned;
use serenity::{
    builder::{
        CreateEmbed,
        CreateMessage,
    },
    http::Http,
};

use crate::{
    models::{
        launches::{
            LaunchData,
            LaunchStatus,
        },
        reminders::{
            GuildSettings,
            UserSettings,
        },
    },
    utils::default_embed,
};

async fn get_toggled<T>(db: &Database, collection: &str, toggled: &str) -> MongoResult<Vec<T>>
where
    T: DeserializeOwned,
{
    db.collection(collection)
        .find(doc! {toggled: true}, None)
        .await?
        .collect::<Vec<MongoResult<Document>>>()
        .await
        .into_iter()
        .collect::<MongoResult<Vec<Document>>>()?
        .into_iter()
        .map(|d| bson::from_document(d).map_err(|e| e.into()))
        .collect()
}

pub async fn notify_scrub(http: Arc<Http>, db: Database, scrub: LaunchData) {
    let user_settings: Vec<UserSettings> =
        if let Ok(settings) = get_toggled(&db, "user_settings", "scrub_notifications").await {
            settings
        } else {
            Vec::new()
        };

    let guild_settings: Vec<GuildSettings> =
        if let Ok(settings) = get_toggled(&db, "guild_settings", "scrub_notifications").await {
            settings
        } else {
            Vec::new()
        };

    stream::iter(user_settings)
        .filter_map(|settings| {
            let http = http.clone();
            async move { settings.user.create_dm_channel(&http).await.ok() }
        })
        .map(|dm| {
            dm.id.send_message(&http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    default_embed(
                        e,
                        &format!(
                            "The launch of {} on a {} is now scheduled for {}",
                            scrub.payload,
                            scrub.vehicle,
                            scrub.net.format("%d %B, %Y; %H:%m:%S UTC").to_string()
                        ),
                        false,
                    )
                })
            })
        })
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;

    stream::iter(guild_settings)
        .filter_map(|settings| future::ready(settings.notifications_channel.map(|c| (c, settings))))
        .map(|(c, settings)| {
            let mentions = settings
                .mentions
                .iter()
                .fold(String::new(), |acc, mention| {
                    acc + &format!(" <@&{}>", mention.as_u64())
                });

            (
                c,
                (settings.mention_others && !mentions.is_empty()).then(|| mentions),
            )
        })
        .map(|(dm, mentions)| {
            dm.send_message(&http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    default_embed(
                        e,
                        &format!(
                            "The launch of {} on a {} is now scheduled for {}",
                            scrub.payload,
                            scrub.vehicle,
                            scrub.net.format("%d %B, %Y; %H:%m:%S UTC").to_string()
                        ),
                        false,
                    )
                });

                if mentions.is_some() {
                    m.content(mentions.unwrap());
                }

                m
            })
        })
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;
}

pub async fn notify_outcome(http: Arc<Http>, db: Database, finished: LaunchData) {
    let user_settings: Vec<UserSettings> =
        if let Ok(settings) = get_toggled(&db, "user_settings", "outcome_notifications").await {
            settings
        } else {
            Vec::new()
        };

    let guild_settings: Vec<GuildSettings> =
        if let Ok(settings) = get_toggled(&db, "guild_settings", "outcome_notifications").await {
            settings
        } else {
            Vec::new()
        };

    stream::iter(user_settings)
        .filter_map(|settings| {
            let http = http.clone();
            async move { settings.user.create_dm_channel(&http).await.ok() }
        })
        .map(|dm| {
            dm.id.send_message(&http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    default_embed(
                        e,
                        &format!(
                            "The launch of {} on a {} has completed with a status of {}",
                            finished.payload,
                            finished.vehicle,
                            finished.status.as_str()
                        ),
                        matches!(finished.status, LaunchStatus::Success),
                    )
                })
            })
        })
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;

    stream::iter(guild_settings)
        .filter_map(|settings| future::ready(settings.notifications_channel.map(|c| (c, settings))))
        .map(|(c, settings)| {
            let mentions = settings
                .mentions
                .iter()
                .fold(String::new(), |acc, mention| {
                    acc + &format!(" <@&{}>", mention.as_u64())
                });

            (
                c,
                (settings.mention_others && !mentions.is_empty()).then(|| mentions),
            )
        })
        .map(|(dm, mentions)| {
            dm.send_message(&http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    default_embed(
                        e,
                        &format!(
                            "The launch of {} on a {} has completed with a status of {}",
                            finished.payload,
                            finished.vehicle,
                            finished.status.as_str()
                        ),
                        matches!(finished.status, LaunchStatus::Success),
                    )
                });

                if mentions.is_some() {
                    m.content(mentions.unwrap());
                }

                m
            })
        })
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;
}

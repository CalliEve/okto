use std::sync::Arc;

use futures::StreamExt;
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
        launches::LaunchData,
        reminders::{
            GuildSettings,
            UserSettings,
        },
    },
    utils::{
        debug_log,
        default_embed,
    },
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

    debug_log(
        &http,
        &format!(
            "sending srub notification for {} off to subscribers\nusers: {}\nchannels: {}",
            scrub.payload,
            user_settings.len(),
            guild_settings.len()
        ),
    )
    .await;

    for user in user_settings {
        if let Ok(dm) = user.user.create_dm_channel(&http).await {
            let _ = dm
                .send_message(&http, |m: &mut CreateMessage| {
                    m.embed(|e: &mut CreateEmbed| {
                        default_embed(
                            e,
                            &format!(
                                "launch of {} on a {} has been delayed to {}",
                                scrub.payload,
                                scrub.vehicle,
                                scrub.net.format("%d %B, %Y; %H:%m:%S UTC").to_string()
                            ),
                            false,
                        )
                    })
                })
                .await;
        }
    }

    for guild in guild_settings {
        if let Some(chan) = guild.notifications_channel {
            let _ = chan
                .send_message(&http, |m: &mut CreateMessage| {
                    m.embed(|e: &mut CreateEmbed| {
                        default_embed(
                            e,
                            &format!(
                                "launch of {} on a {} has been delayed to {}",
                                scrub.payload,
                                scrub.vehicle,
                                scrub.net.format("%d %B, %Y; %H:%m:%S UTC").to_string()
                            ),
                            false,
                        )
                    })
                })
                .await;
        }
    }
}

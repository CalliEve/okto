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
    model::{
        channel::Message,
        id::ChannelId,
        Colour,
        Timestamp,
    },
    Error as SerenityError,
};

use super::filtering::passes_filters;
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

async fn get_toggled<T>(db: &Database, collection: &str, toggled: &str) -> Vec<T>
where
    T: DeserializeOwned,
{
    let Ok(cursor) = db
        .collection(collection)
        .find(doc! {toggled: true}, None)
        .await
    else {
        return Vec::new();
    };

    let Ok(documents) = cursor
        .collect::<Vec<MongoResult<Document>>>()
        .await
        .into_iter()
        .collect::<MongoResult<Vec<Document>>>()
    else {
        return Vec::new();
    };

    if let Ok(settings) = documents
        .into_iter()
        .map(bson::from_document)
        .collect()
    {
        settings
    } else {
        Vec::new()
    }
}

fn get_mentions(settings: &GuildSettings) -> Option<String> {
    let mentions = settings
        .mentions
        .iter()
        .fold(String::new(), |acc, mention| {
            acc + &format!(" <@&{}>", mention.get())
        });

    (settings.mention_others && !mentions.is_empty()).then_some(mentions)
}

async fn send_user_notification<'r>(
    http: &'r Arc<Http>,
    all_settings: Vec<UserSettings>,
    launch: &'r LaunchData,
    embed: &'r CreateEmbed,
) {
    stream::iter(all_settings)
        .filter(|settings| future::ready(passes_filters(settings, launch)))
        .filter_map(|settings| {
            let http = http.clone();
            async move {
                settings
                    .user
                    .create_dm_channel(&http)
                    .await
                    .ok()
            }
        })
        .map(|channel| send_message(http, channel.id, None, embed.clone()))
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;
}

async fn send_guild_notification<'r>(
    http: &'r Arc<Http>,
    all_settings: Vec<GuildSettings>,
    launch: &'r LaunchData,
    embed: &'r CreateEmbed,
) {
    stream::iter(all_settings)
        .filter(|settings| future::ready(passes_filters(settings, launch)))
        .filter_map(|settings| {
            future::ready(
                settings
                    .notifications_channel
                    .map(|c| (c, settings)),
            )
        })
        .map(|(c, settings)| (c, get_mentions(&settings)))
        .map(|(channel, mentions)| send_message(http, channel, mentions, embed.clone()))
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;
}

pub async fn notify_scrub(http: Arc<Http>, db: Database, old: LaunchData, new: LaunchData) {
    let user_settings: Vec<UserSettings> = get_toggled(
        &db,
        "user_settings",
        "scrub_notifications",
    )
    .await;

    let guild_settings: Vec<GuildSettings> = get_toggled(
        &db,
        "guild_settings",
        "scrub_notifications",
    )
    .await;

    let embed = scrub_embed(&old, &new);

    send_user_notification(&http, user_settings, &new, &embed).await;

    send_guild_notification(&http, guild_settings, &new, &embed).await;
}

async fn send_message(
    http: &Arc<Http>,
    channel: ChannelId,
    mentions_opt: Option<String>,
    embed: CreateEmbed,
) -> Result<Message, SerenityError> {
    let mut message = CreateMessage::new().embed(embed);

    if let Some(mentions) = mentions_opt {
        message = message.content(mentions);
    }

    channel
        .send_message(http, message)
        .await
}

fn scrub_embed<'r>(old: &'r LaunchData, new: &'r LaunchData) -> CreateEmbed {
    default_embed(
        &format!(
            "The launch of {} on a **{}** is now scheduled for <t:{}>{} instead of <t:{}>{}",
            new.payload,
            new.vehicle,
            new.net
                .timestamp(),
            if new.status == LaunchStatus::Tbd {
                " (TBD)"
            } else {
                ""
            },
            old.net
                .timestamp(),
            if old.status == LaunchStatus::Tbd {
                " (TBD)"
            } else {
                ""
            },
        ),
        false,
    )
    .timestamp(
        Timestamp::from_unix_timestamp(
            new.net
                .timestamp(),
        )
        .expect("Invalid timestamp"),
    )
}

pub async fn notify_outcome(http: Arc<Http>, db: Database, finished: LaunchData) {
    let user_settings: Vec<UserSettings> = get_toggled(
        &db,
        "user_settings",
        "outcome_notifications",
    )
    .await;

    let guild_settings: Vec<GuildSettings> = get_toggled(
        &db,
        "guild_settings",
        "outcome_notifications",
    )
    .await;

    let embed = outcome_embed(&finished);

    send_user_notification(&http, user_settings, &finished, &embed).await;

    send_guild_notification(&http, guild_settings, &finished, &embed).await;
}

fn outcome_embed(finished: &LaunchData) -> CreateEmbed {
    default_embed(
        &format!(
            "The launch of {} on a {} has completed with a status of **{}**!",
            &finished.payload,
            &finished.vehicle,
            finished
                .status
                .as_str()
        ),
        true,
    )
    .color(
        if matches!(finished.status, LaunchStatus::Success) {
            Colour::FOOYOO
        } else if matches!(
            finished.status,
            LaunchStatus::PartialFailure
        ) {
            Colour::ORANGE
        } else {
            Colour::RED
        },
    )
}

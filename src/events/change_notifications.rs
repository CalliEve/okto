use std::{
    future::Future,
    sync::Arc,
};

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
    },
    Error as SerenityError,
};

use crate::{
    models::{
        launches::{
            LaunchData,
            LaunchStatus,
        },
        reminders::{
            GuildSettings,
            ReminderSettings,
            UserSettings,
        },
    },
    utils::{
        constants::LAUNCH_AGENCIES,
        default_embed,
    },
};

async fn get_toggled<T>(db: &Database, collection: &str, toggled: &str) -> Vec<T>
where
    T: DeserializeOwned,
{
    let cursor = if let Ok(cursor) = db
        .collection(collection)
        .find(doc! {toggled: true}, None)
        .await
    {
        cursor
    } else {
        return Vec::new();
    };

    let documents = if let Ok(docs) = cursor
        .collect::<Vec<MongoResult<Document>>>()
        .await
        .into_iter()
        .collect::<MongoResult<Vec<Document>>>()
    {
        docs
    } else {
        return Vec::new();
    };

    if let Ok(settings) = documents.into_iter().map(bson::from_document).collect() {
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
            acc + &format!(" <@&{}>", mention.as_u64())
        });

    (settings.mention_others && !mentions.is_empty()).then(|| mentions)
}

fn passes_filters<T>(settings: &T, l: &LaunchData) -> bool
where
    T: ReminderSettings,
{
    !settings
        .get_filters()
        .iter()
        .filter_map(|filter| LAUNCH_AGENCIES.get(filter.as_str()))
        .any(|agency| *agency == l.lsp)
}

async fn send_user_notification<'r, F>(
    http: &'r Arc<Http>,
    all_settings: Vec<UserSettings>,
    launch: &'r LaunchData,
    message_maker: fn(
        http: &'r Arc<Http>,
        scrub: &'r LaunchData,
        channel: ChannelId,
        mentions_opt: Option<String>,
    ) -> F,
) where
    F: Future<Output = Result<Message, SerenityError>>,
{
    stream::iter(all_settings)
        .filter(|settings| future::ready(passes_filters(settings, &launch)))
        .filter_map(|settings| {
            let http = http.clone();
            async move { settings.user.create_dm_channel(&http).await.ok() }
        })
        .map(|channel| message_maker(http, launch, channel.id, None))
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;
}

async fn send_guild_notification<'r, F>(
    http: &'r Arc<Http>,
    all_settings: Vec<GuildSettings>,
    launch: &'r LaunchData,
    message_maker: fn(
        http: &'r Arc<Http>,
        scrub: &'r LaunchData,
        channel: ChannelId,
        mentions_opt: Option<String>,
    ) -> F,
) where
    F: Future<Output = Result<Message, SerenityError>>,
{
    stream::iter(all_settings)
        .filter(|settings| future::ready(passes_filters(settings, launch)))
        .filter_map(|settings| future::ready(settings.notifications_channel.map(|c| (c, settings))))
        .map(|(c, settings)| (c, get_mentions(&settings)))
        .map(|(channel, mentions)| message_maker(http, launch, channel, mentions))
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;
}

pub async fn notify_scrub(http: Arc<Http>, db: Database, scrub: LaunchData) {
    let user_settings: Vec<UserSettings> =
        get_toggled(&db, "user_settings", "scrub_notifications").await;

    let guild_settings: Vec<GuildSettings> =
        get_toggled(&db, "guild_settings", "scrub_notifications").await;

    send_user_notification(&http, user_settings, &scrub, scrub_message).await;

    send_guild_notification(&http, guild_settings, &scrub, scrub_message).await;
}

async fn scrub_message<'r>(
    http: &'r Arc<Http>,
    scrub: &'r LaunchData,
    channel: ChannelId,
    mentions_opt: Option<String>,
) -> Result<Message, SerenityError> {
    channel
        .send_message(http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                default_embed(
                    e,
                    &format!(
                        "The launch of {} on a **{}** is now scheduled for **{}**",
                        scrub.payload,
                        scrub.vehicle,
                        scrub.net.format("%d %B, %Y; %H:%m:%S UTC").to_string()
                    ),
                    false,
                );

                e.timestamp(scrub.net.format("%Y-%m-%dT%H:%M:%S").to_string())
            });

            if let Some(mentions) = mentions_opt {
                m.content(mentions);
            }

            m
        })
        .await
}

pub async fn notify_outcome(http: Arc<Http>, db: Database, finished: LaunchData) {
    let user_settings: Vec<UserSettings> =
        get_toggled(&db, "user_settings", "outcome_notifications").await;

    let guild_settings: Vec<GuildSettings> =
        get_toggled(&db, "guild_settings", "outcome_notifications").await;

    send_user_notification(&http, user_settings, &finished, outcome_message).await;

    send_guild_notification(&http, guild_settings, &finished, outcome_message).await;
}

async fn outcome_message<'r>(
    http: &'r Arc<Http>,
    finished: &'r LaunchData,
    channel: ChannelId,
    mentions_opt: Option<String>,
) -> Result<Message, SerenityError> {
    channel
        .send_message(http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                default_embed(
                    e,
                    &format!(
                        "The launch of {} on a {} has completed with a status of **{}**!",
                        &finished.payload,
                        &finished.vehicle,
                        finished.status.as_str()
                    ),
                    matches!(finished.status, LaunchStatus::Success),
                )
            });

            if let Some(mentions) = mentions_opt {
                m.content(mentions);
            }

            m
        })
        .await
}

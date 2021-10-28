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
    },
    utils::Colour,
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
        && (settings
            .get_allow_filters()
            .is_empty()
            || settings
                .get_allow_filters()
                .iter()
                .filter_map(|filter| LAUNCH_AGENCIES.get(filter.as_str()))
                .any(|agency| *agency == l.lsp))
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
        .map(|channel| send_message(http, channel.id, None, embed))
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
        .map(|(channel, mentions)| send_message(http, channel, mentions, embed))
        .collect::<FuturesUnordered<_>>()
        .await
        .collect::<Vec<_>>()
        .await;
}

pub async fn notify_scrub(http: Arc<Http>, db: Database, old: LaunchData, new: LaunchData) {
    let user_settings: Vec<UserSettings> =
        get_toggled(&db, "user_settings", "scrub_notifications").await;

    let guild_settings: Vec<GuildSettings> =
        get_toggled(&db, "guild_settings", "scrub_notifications").await;

    let embed = scrub_embed(&old, &new);

    send_user_notification(&http, user_settings, &new, &embed).await;

    send_guild_notification(&http, guild_settings, &new, &embed).await;
}

async fn send_message<'r>(
    http: &'r Arc<Http>,
    channel: ChannelId,
    mentions_opt: Option<String>,
    embed: &'r CreateEmbed,
) -> Result<Message, SerenityError> {
    channel
        .send_message(http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                *e = embed.clone();
                e
            });

            if let Some(mentions) = mentions_opt {
                m.content(mentions);
            }

            m
        })
        .await
}

fn scrub_embed<'r>(old: &'r LaunchData, new: &'r LaunchData) -> CreateEmbed {
    let mut e = CreateEmbed::default();

    default_embed(
        &mut e,
        &format!(
            "The launch of {} on a **{}** is now scheduled for <t:{}> instead of <t:{}>",
            new.payload,
            new.vehicle,
            new.net
                .timestamp(),
            old.net
                .timestamp()
        ),
        false,
    );

    e.timestamp(
        new.net
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string(),
    );

    e
}

pub async fn notify_outcome(http: Arc<Http>, db: Database, finished: LaunchData) {
    let user_settings: Vec<UserSettings> =
        get_toggled(&db, "user_settings", "outcome_notifications").await;

    let guild_settings: Vec<GuildSettings> =
        get_toggled(&db, "guild_settings", "outcome_notifications").await;

    let embed = outcome_embed(&finished);

    send_user_notification(&http, user_settings, &finished, &embed).await;

    send_guild_notification(&http, guild_settings, &finished, &embed).await;
}

fn outcome_embed(finished: &LaunchData) -> CreateEmbed {
    let mut e = CreateEmbed::default();

    default_embed(
        &mut e,
        &format!(
            "The launch of {} on a {} has completed with a status of **{}**!",
            &finished.payload,
            &finished.vehicle,
            finished
                .status
                .as_str()
        ),
        true,
    );

    e.color(
        if matches!(finished.status, LaunchStatus::Success) {
            Colour::FOOYOO
        } else if matches!(finished.status, LaunchStatus::PartialFailure) {
            Colour::ORANGE
        } else {
            Colour::RED
        },
    );

    e
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use chrono::NaiveDateTime;
    use serenity::model::id::{
        ChannelId,
        GuildId,
    };

    use super::*;

    fn create_fake_launches() -> Vec<LaunchData> {
        vec![
            LaunchData {
                id: 1,
                ll_id: String::new(),
                launch_name: "Falcon 9 Block 5".into(),
                status: LaunchStatus::Go,
                payload: "SpX USCV-3 (NASA Crew Flight 3)".into(),
                vid_urls: vec![],
                vehicle: "Falcon 9 Block 5".into(),
                location: "Launch Complex 39A".into(),
                rocket_img: None,
                net: NaiveDateTime::from_timestamp(1635409251, 0),
                launch_window: Duration::seconds(60),
                mission_type: String::new(),
                mission_description: String::new(),
                lsp: "SpaceX".into(),
            },
            LaunchData {
                id: 2,
                ll_id: String::new(),
                launch_name: "Atlas V 551".into(),
                status: LaunchStatus::Go,
                payload: "STP-3".into(),
                vid_urls: vec![],
                vehicle: "Atlas V 551".into(),
                location: "Space Launch Complex 41".into(),
                rocket_img: None,
                net: NaiveDateTime::from_timestamp(1635409251, 0),
                launch_window: Duration::seconds(60),
                mission_type: String::new(),
                mission_description: String::new(),
                lsp: "United Launch Alliance".into(),
            },
        ]
    }

    #[test]
    fn no_filters() {
        let launches = create_fake_launches();
        let settings = GuildSettings {
            guild: GuildId(429307670730637312),
            filters: vec![],
            allow_filters: vec![],
            mentions: vec![],
            scrub_notifications: true,
            outcome_notifications: true,
            mention_others: true,
            notifications_channel: Some(ChannelId(429307774804033536)),
        };

        assert!(passes_filters(&settings, &launches[0]));
        assert!(passes_filters(&settings, &launches[1]));
    }

    #[test]
    fn block_filters() {
        let launches = create_fake_launches();
        let settings = GuildSettings {
            guild: GuildId(429307670730637312),
            filters: vec!["ula".into()],
            allow_filters: vec![],
            mentions: vec![],
            scrub_notifications: true,
            outcome_notifications: true,
            mention_others: true,
            notifications_channel: Some(ChannelId(429307774804033536)),
        };

        assert!(passes_filters(&settings, &launches[0]));
        assert!(!passes_filters(&settings, &launches[1]));
    }

    #[test]
    fn allow_filters() {
        let launches = create_fake_launches();
        let settings = GuildSettings {
            guild: GuildId(429307670730637312),
            filters: vec![],
            allow_filters: vec!["ula".into()],
            mentions: vec![],
            scrub_notifications: true,
            outcome_notifications: true,
            mention_others: true,
            notifications_channel: Some(ChannelId(429307774804033536)),
        };

        assert!(!passes_filters(&settings, &launches[0]));
        assert!(passes_filters(&settings, &launches[1]));
    }
}

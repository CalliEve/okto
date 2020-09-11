use mongodb::{bson::doc, options::UpdateOptions};
use serenity::{
    builder::{CreateEmbed, CreateMessage},
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::channel::{Channel, Message},
    prelude::{Context, RwLock},
};

use chrono::Duration;

use std::str::FromStr;
use std::sync::Arc;

use super::reminders::{add_reminder, ID};
use crate::events::statefulembed::EmbedSession;
use crate::{models::caches::DatabaseKey, models::legacy::*, utils::default_embed};

#[group]
#[commands(setprefix, loaddb)]
struct Settings;

#[command]
#[required_permissions(MANAGE_GUILD)]
#[only_in(guild)]
fn setprefix(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let prefix = if let Some(prefix) = args.current() {
        prefix
    } else {
        ";"
    };

    let db = if let Some(db) = ctx.data.read().get::<DatabaseKey>() {
        db.clone()
    } else {
        return Err("No database found".into());
    };

    let res = db.collection("general_settings").update_one(
        doc! {"guild": msg.guild_id.unwrap().0},
        doc! {
            "prefix": &prefix
        },
        Some(UpdateOptions::builder().upsert(true).build()),
    );

    if res.is_ok() {
        let res = res.unwrap();
        if res.modified_count == 0 && res.upserted_id.is_none() {
            return Err("No document got updated when changing the guild prefix".into());
        }

        msg.channel_id
            .send_message(&ctx.http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    default_embed(
                        e,
                        &format!("My prefix in this guild has been updated to {}", prefix),
                        true,
                    )
                })
            })?;
    } else {
        res?;
    }

    Ok(())
}

#[command]
#[owners_only]
fn loaddb(ctx: &mut Context, msg: &Message) -> CommandResult {
    if msg.author.id != 247745860979392512 {
        return Err("not allowed".into());
    }

    let db = if let Some(db) = ctx.data.read().get::<DatabaseKey>() {
        db.clone()
    } else {
        return Err("No database found".into());
    };

    let file = msg
        .attachments
        .first()
        .ok_or("no attachments")?
        .download()?;
    let legacy_settings: Legacy = serde_json::from_slice(&file)?;

    println!("{:?}", serde_json::to_string(&legacy_settings));

    for setting in &legacy_settings.settings {
        let res = db.collection("general_settings").update_one(
            doc! {"guild": &setting.id},
            doc! {
                "prefix": &setting.prefix
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        );

        if res.is_err() {
            return Err(format!("{}", res.unwrap_err()).into());
        }
    }

    let ses = Arc::new(RwLock::new(EmbedSession::new(
        &ctx,
        msg.channel_id,
        msg.author.id,
    )));

    for user_reminder in &legacy_settings.users {
        for dur in user_reminder.to_vec() {
            add_reminder(
                &ses,
                ID::User(u64::from_str(&user_reminder.id)?.into()),
                get_dur(dur),
            )
        }
    }

    for channel_reminder in &legacy_settings.channels {
        let channel_opt = ctx
            .cache
            .read()
            .channel(u64::from_str(&channel_reminder.id)?);

        if let Some(Channel::Guild(channel)) = channel_opt {
            for dur in channel_reminder.to_vec() {
                add_reminder(
                    &ses,
                    ID::Channel((channel.read().id, channel.read().guild_id)),
                    get_dur(dur),
                )
            }
        }
    }

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                default_embed(
                    e,
                    &format!("Legacy Database has been loaded from json file"),
                    true,
                )
            })
        })?;

    Ok(())
}

fn get_dur(text: &str) -> Duration {
    match text {
        "24h" => Duration::hours(24),
        "12h" => Duration::hours(12),
        "6h" => Duration::hours(6),
        "3h" => Duration::hours(3),
        "1h" => Duration::hours(1),
        "30m" => Duration::minutes(30),
        "15m" => Duration::minutes(15),
        "5m" => Duration::minutes(5),
        "1m" => Duration::minutes(1),
        _ => Duration::seconds(0),
    }
}

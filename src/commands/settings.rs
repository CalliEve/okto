use mongodb::{
    bson::{doc, document::Document},
    options::UpdateOptions,
};
use serenity::{
    builder::{CreateEmbed, CreateMessage},
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::channel::Message,
    prelude::Context,
};

use crate::{models::caches::DatabaseKey, utils::default_embed};

#[group]
#[commands(setprefix)]
struct Settings;

#[command]
#[required_permissions(MANAGE_GUILD)]
#[only_in(guild)]
#[usage("Provide new prefix to use or no arguments to have it reset to ;")]
#[description("Sets prefix to be listened to in this guild")]
async fn setprefix(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let prefix = args.current().map_or(";", |p| p);

    let db = if let Some(db) = ctx.data.read().await.get::<DatabaseKey>() {
        db.clone()
    } else {
        return Err("No database found".into());
    };

    let res = db
        .collection::<Document>("general_settings")
        .update_one(
            doc! {"guild": msg.guild_id.unwrap().0 as i64},
            doc! {
                "$set": {
                    "prefix": prefix,
                },
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        )
        .await?;

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
        })
        .await?;

    Ok(())
}

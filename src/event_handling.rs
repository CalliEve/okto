use serenity::{
    model::{
        channel::{Message, Reaction},
        id::{ChannelId, GuildId, MessageId},
    },
    prelude::{Context, EventHandler},
};

use crate::{
    events::{
        statefulembed::{on_message_delete as embed_delete, on_reaction_add as embed_reactions},
        waitfor::{waitfor_message, waitfor_reaction},
    },
    launch_tracking::launch_tracking,
    models::caches::{DatabaseKey, LaunchesCacheKey},
    reminder_tracking::reminder_tracking,
};

pub struct Handler;

impl EventHandler for Handler {
    fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        {
            let cache = ctx.cache.read();
            println!(
                "############\n\
                Logged in as:\n{}\n{}\n\
                guilds: {}\n\
                Users: {}\n\
                ############",
                cache.user.name,
                cache.user.id,
                cache.all_guilds().len(),
                cache.users.len()
            )
        }

        if let Some(launches_cache) = ctx.data.read().get::<LaunchesCacheKey>() {
            let launches_cache_clone = launches_cache.clone();
            std::thread::spawn(|| launch_tracking(launches_cache_clone));

            if let Some(db) = ctx.data.read().get::<DatabaseKey>() {
                let launches_cache_clone = launches_cache.clone();
                let http_clone = ctx.http.clone();
                let db_clone = db.clone();
                std::thread::spawn(|| {
                    reminder_tracking(http_clone, launches_cache_clone, db_clone)
                });
            }
        }
    }

    fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        waitfor_reaction(&ctx, add_reaction.clone());
        embed_reactions(&ctx, add_reaction.clone());
    }

    fn message_delete(&self, ctx: Context, _channel_id: ChannelId, deleted_message_id: MessageId) {
        embed_delete(&ctx, deleted_message_id)
    }

    fn message(&self, ctx: Context, message: Message) {
        waitfor_message(&ctx, message)
    }
}

use serenity::{
    model::{
        channel::Reaction,
        id::{ChannelId, GuildId, MessageId},
    },
    prelude::{Context, EventHandler},
};

use crate::{
    events::statefulembed::{
        on_message_delete as embed_delete,
        on_reaction_add as embed_reactions,
    },
    launch_tracking::launch_tracking,
    models::caches::LaunchesCacheKey,
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
        }
    }

    fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        embed_reactions(ctx, add_reaction)
    }

    fn message_delete(&self, ctx: Context, channel_id: ChannelId, deleted_message_id: MessageId) {
        embed_delete(ctx, deleted_message_id)
    }
}

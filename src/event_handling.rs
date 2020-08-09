use num_cpus;
use serenity::{
    model::{
        channel::{Message, Reaction},
        id::{ChannelId, GuildId, MessageId},
    },
    prelude::{Context, EventHandler},
};

use crate::events::{
    statefulembed::{on_message_delete as embed_delete, on_reaction_add as embed_reactions},
    waitfor::{waitfor_message, waitfor_reaction},
};

pub struct Handler;

impl EventHandler for Handler {
    fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let cache = ctx.cache.read();
        println!(
            "############
            Logged in as:\n{} - {}
            guilds: {}
            Users: {}
            CPUs: {}
            ############",
            cache.user.name,
            cache.user.id,
            cache.all_guilds().len(),
            cache.users.len(),
            num_cpus::get()
        )
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

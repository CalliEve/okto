use std::thread;
use std::time::Duration;

use num_cpus;
use serenity::{
    model::{
        channel::{Message, Reaction},
        id::{ChannelId, GuildId, MessageId},
        prelude::Activity,
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
        {
            let cache = ctx.cache.read();
            println!(
                "############\n\
            Logged in as: {} - {}\n\
            guilds: {}\n\
            Users: {}\n\
            CPUs: {}\n\
            ############",
                cache.user.name,
                cache.user.id,
                cache.all_guilds().len(),
                cache.users.len(),
                num_cpus::get()
            );

            if let Some(channel) = cache.guild_channel(448224720177856513) {
                let _ = channel.read().send_message(&ctx.http, |m| {
                    m.content(format!(
                        "**OKTO Beta** restarted\nServing {} servers with {} members total",
                        cache.all_guilds().len(),
                        cache.users.len()
                    ))
                });
            }
        }

        thread::spawn(move || loop {
            {
                let cache = ctx.cache.read();
                ctx.shard.set_activity(Some(Activity::listening(&format!(
                    "{} servers",
                    cache.all_guilds().len()
                ))));
            }
            thread::sleep(Duration::from_secs(300));
        });
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

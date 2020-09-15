use std::thread;
use std::time::Duration;

use num_cpus;
use serenity::{
    async_trait,
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

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!(
            "############\n\
            Logged in as: {} - {}\n\
            guilds: {}\n\
            CPUs: {}\n\
            ############",
            ctx.cache.current_user().await.name,
            ctx.cache.current_user().await.id,
            ctx.cache.guilds().await.len(),
            num_cpus::get()
        );

        if let Some(channel) = ctx.cache.guild_channel(448224720177856513).await {
            let content = format!(
                "**OKTO** restarted\nServing {} servers with {} members total",
                ctx.cache.guilds().await.len(),
                ctx.cache.users().await.len()
            );
            let _ = channel.send_message(&ctx.http, |m| m.content(content));
        }

        tokio::spawn(async move {
            loop {
                {
                    let status = format!("{} servers", ctx.cache.guilds().await.len());
                    ctx.shard.set_activity(Some(Activity::listening(&status)));
                }
                thread::sleep(Duration::from_secs(300));
            }
        });
    }

    async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
        waitfor_reaction(&ctx, add_reaction.clone()).await;
        embed_reactions(&ctx, add_reaction.clone()).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        _channel_id: ChannelId,
        deleted_message_id: MessageId,
    ) {
        embed_delete(&ctx, deleted_message_id).await
    }

    async fn message(&self, ctx: Context, message: Message) {
        waitfor_message(&ctx, message).await
    }
}

use std::time::Duration;

use serenity::{
    async_trait,
    model::{
        channel::{Message, Reaction},
        guild::{Guild, PartialGuild},
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
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        println!(
            "############\n\
            Logged in as: {} - {}\n\
            guilds: {}\n\
            CPUs: {}\n\
            ############",
            ctx.cache.current_user().await.name,
            ctx.cache.current_user().await.id,
            guilds.len(),
            num_cpus::get()
        );

        if let Some(channel) = ctx.cache.guild_channel(448224720177856513).await {
            let content = format!(
                "**OKTO** restarted\nServing {} servers",
                ctx.cache.guilds().await.len(),
            );
            let _ = channel
                .send_message(&ctx.http, |m| m.content(content))
                .await;
        }

        tokio::spawn(async move {
            loop {
                {
                    let status = format!("{} servers", ctx.cache.guilds().await.len());
                    ctx.shard.set_activity(Some(Activity::listening(&status)));
                }
                tokio::time::delay_for(Duration::from_secs(300)).await;
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

    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if is_new {
            if let Some(channel) = ctx.cache.guild_channel(755401788294955070).await {
                let content = format!(
                    "Joined a new guild: {} ({})\nIt has {} members",
                    guild.name, guild.id, guild.member_count
                );
                let _ = channel
                    .send_message(&ctx.http, |m| m.content(content))
                    .await;
            }
        }
    }

    async fn guild_delete(&self, ctx: Context, _incomplete: PartialGuild, full: Option<Guild>) {
        if let Some(guild) = full {
            if let Some(channel) = ctx.cache.guild_channel(755401788294955070).await {
                let content = format!("Left the following guild: {} ({})", guild.name, guild.id);
                let _ = channel
                    .send_message(&ctx.http, |m| m.content(content))
                    .await;
            }
        }
    }
}

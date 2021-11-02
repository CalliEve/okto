use std::{
    collections::HashMap,
    time::Duration,
};

use reqwest::header::AUTHORIZATION;
use serenity::{
    async_trait,
    model::{
        channel::{
            Message,
            Reaction,
        },
        gateway::Ready,
        guild::{
            Guild,
            GuildUnavailable,
        },
        id::{
            ChannelId,
            GuildId,
            MessageId,
        },
        prelude::Activity,
    },
    prelude::{
        Context,
        EventHandler,
    },
};

use crate::{
    events::{
        statefulembed::{
            on_message_delete as embed_delete,
            on_reaction_add as embed_reactions,
        },
        waitfor::{
            waitfor_message,
            waitfor_reaction,
        },
    },
    utils::constants::{
        DEFAULT_CLIENT,
        TOPGG_TOKEN,
    },
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!(
            "############\n\
            Logged in as: {} - {}\n\
            guilds: {}\n\
            CPUs: {}\n\
            ############",
            ready
                .user
                .name,
            ready
                .user
                .id,
            ready
                .guilds
                .len(),
            num_cpus::get()
        );

        let content = format!(
            "**OKTO** restarted\nServing {} servers",
            ready
                .guilds
                .len(),
        );
        let _ = ChannelId(448224720177856513)
            .send_message(&ctx.http, |m| m.content(content))
            .await;
        
        let status = format!("{} servers", ready.guilds.len());
        ctx.set_activity(Activity::listening(&status))
            .await;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await;
                {
                    let amount: usize = ctx
                        .cache
                        .guild_count()
                        .await;
                    let status = format!("{} servers", amount);
                    ctx.set_activity(Activity::listening(&status))
                        .await;

                    let mut map = HashMap::new();
                    map.insert("server_count", amount);
                    let _ = DEFAULT_CLIENT
                        .post(
                            format!(
                                "https://top.gg/api/bots/{}/stats",
                                ready
                                    .user
                                    .id
                            )
                            .as_str(),
                        )
                        .header(AUTHORIZATION, TOPGG_TOKEN.as_str())
                        .json(&map)
                        .send()
                        .await;
                }
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
        _guild_id: Option<GuildId>,
    ) {
        embed_delete(&ctx, deleted_message_id).await
    }

    async fn message(&self, ctx: Context, message: Message) {
        waitfor_message(&ctx, message).await
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if is_new {
            if let Some(channel) = ctx
                .cache
                .guild_channel(755401788294955070)
                .await
            {
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

    async fn guild_delete(&self, ctx: Context, incomplete: GuildUnavailable, _full: Option<Guild>) {
        if !incomplete.unavailable {
            if let Some(channel) = ctx
                .cache
                .guild_channel(755401788294955070)
                .await
            {
                let content = format!("Left the following guild: {}", incomplete.id);
                let _ = channel
                    .send_message(&ctx.http, |m| m.content(content))
                    .await;
            }
        }
    }
}

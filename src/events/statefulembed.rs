use std::sync::Arc;

use futures::future::BoxFuture;
use serenity::{
    builder::CreateEmbed,
    cache::Cache,
    http::Http,
    model::{
        channel::{Message, Reaction, ReactionType},
        id::{ChannelId, MessageId, UserId},
    },
    prelude::{Context, RwLock, TypeMap},
    Error,
};

use crate::models::caches::EmbedSessionsKey;

type Handler = dyn Fn() -> BoxFuture<'static, ()> + Send + Sync;

#[derive(Clone)]
pub struct StatefulOption {
    pub emoji: ReactionType,
    pub handler: Arc<Box<Handler>>,
}

#[derive(Clone)]
pub struct StatefulEmbed {
    pub inner: CreateEmbed,
    pub session: Arc<RwLock<EmbedSession>>,
    pub options: Vec<StatefulOption>,
}

impl StatefulEmbed {
    #[allow(dead_code)]
    pub fn new(session: Arc<RwLock<EmbedSession>>) -> Self {
        Self {
            inner: CreateEmbed::default(),
            session,
            options: Vec::new(),
        }
    }

    pub fn new_with<F>(session: Arc<RwLock<EmbedSession>>, f: F) -> Self
    where
        F: FnOnce(&mut CreateEmbed) -> &mut CreateEmbed,
    {
        let mut em = CreateEmbed::default();
        f(&mut em);

        Self {
            inner: em,
            session,
            options: Vec::new(),
        }
    }

    pub fn add_field<F>(
        &mut self,
        name: &str,
        value: &str,
        inline: bool,
        emoji: &ReactionType,
        handler: F,
    ) where
        F: Fn() -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        let full_name = format!("{} {}", emoji.to_string(), name);
        self.inner.field(full_name, value, inline);
        self.options.push(StatefulOption {
            emoji: emoji.clone(),
            handler: Arc::new(Box::new(handler)),
        })
    }

    pub fn add_option<F>(&mut self, emoji: &ReactionType, handler: F)
    where
        F: Fn() -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        self.options.push(StatefulOption {
            emoji: emoji.clone(),
            handler: Arc::new(Box::new(handler)),
        })
    }

    async fn add_reactions(&self) -> serenity::Result<()> {
        let session = self.session.read().await;
        let message: &Message = session
            .message
            .as_ref()
            .ok_or(Error::Other("No message in session"))?;

        let res = message.delete_reactions(&session.http).await;
        if res.is_err() {
            for r in &message.reactions {
                if r.me {
                    let _ = message
                        .channel_id
                        .delete_reaction(
                            &session.http,
                            message.id,
                            Some(message.author.id),
                            r.reaction_type.clone(),
                        )
                        .await;
                }
            }
        }

        for opt in &self.options {
            message.react(&session.http, opt.emoji.clone()).await?;
        }

        Ok(())
    }

    pub async fn show(&self) -> serenity::Result<()> {
        {
            let mut session = self.session.write().await;
            let http = session.http.clone();
            session.set_embed(self.clone());

            if let Some(message) = session.message.as_mut() {
                message
                    .edit(&http, |m| {
                        m.embed(|e: &mut CreateEmbed| {
                            e.0 = self.inner.0.clone();
                            e
                        })
                    })
                    .await?;
            } else {
                let msg = session
                    .channel
                    .send_message(&http, |m| {
                        m.embed(|e: &mut CreateEmbed| {
                            e.0 = self.inner.0.clone();
                            e
                        })
                    })
                    .await?;

                if let Some(embeds) = session.data.write().await.get_mut::<EmbedSessionsKey>() {
                    embeds.insert(msg.id, self.session.clone());
                }

                session.message = Some(msg);
            }
        }

        self.add_reactions().await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct EmbedSession {
    pub current_state: Option<StatefulEmbed>,
    pub message: Option<Message>,
    pub channel: ChannelId,
    pub author: UserId,
    pub http: Arc<Http>,
    pub data: Arc<RwLock<TypeMap>>,
    pub cache: Arc<Cache>,
}

impl EmbedSession {
    pub fn new(ctx: &Context, channel: ChannelId, author: UserId) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            channel,
            author,
            http: ctx.http.clone(),
            current_state: None,
            message: None,
            data: ctx.data.clone(),
            cache: ctx.cache.clone(),
        }))
    }

    fn set_embed(&mut self, em: StatefulEmbed) {
        self.current_state = Some(em)
    }
}

pub async fn on_reaction_add(ctx: &Context, add_reaction: Reaction) {
    if let Some(user_id) = add_reaction.user_id {
        let handler = if let Some(cache) = ctx.data.read().await.get::<EmbedSessionsKey>() {
            if let Some(session_lock) = cache.get(&add_reaction.message_id) {
                let session = session_lock.read().await;
                if session.author != user_id || session.channel != add_reaction.channel_id {
                    return;
                }

                session.current_state.as_ref().and_then(|embed| {
                    let mut handler: Option<Arc<Box<Handler>>> = None;

                    for opt in &embed.options {
                        if opt.emoji == add_reaction.emoji {
                            handler = Some(opt.handler.clone());
                        }
                    }

                    handler
                })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(handler) = handler {
            handler().await;
        }
    }
}

pub async fn on_message_delete(ctx: &Context, message_id: MessageId) {
    if let Some(cache) = ctx.data.write().await.get_mut::<EmbedSessionsKey>() {
        cache.remove(&message_id);
    }
}

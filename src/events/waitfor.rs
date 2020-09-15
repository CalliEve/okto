use std::sync::Arc;

use futures::future::BoxFuture;
use serenity::{
    builder::CreateMessage,
    http::Http,
    model::{
        channel::{Message, Reaction},
        id::{ChannelId, UserId},
    },
    prelude::{Context, RwLock, TypeMap},
    Result as SerenityResult,
};

use crate::models::caches::WaitForKey;

#[derive(Clone, Debug)]
pub enum WaitPayload {
    Message(Message),
    Reaction(Reaction),
}

impl WaitPayload {
    async fn delete(&self, ctx: &Context) -> SerenityResult<()> {
        match self {
            Self::Message(m) => m.delete(ctx).await,
            Self::Reaction(r) => r.delete(ctx).await,
        }
    }
}

type Handler = Arc<Box<dyn Fn(WaitPayload) -> BoxFuture<'static, ()> + Send + Sync>>;

type Filter = Arc<Box<dyn Fn(WaitPayload) -> BoxFuture<'static, bool> + Send + Sync>>;

#[derive(Clone, PartialEq, Debug)]
pub enum WaitType {
    Message,
    Reaction,
}

#[derive(Clone)]
pub struct WaitFor {
    channel: ChannelId,
    user: UserId,
    wait_type: WaitType,
    handler: Handler,
    filter: Option<Filter>,
    message: Option<Message>,
}

#[allow(dead_code)]
impl WaitFor {
    pub fn message<H>(channel: ChannelId, user: UserId, handler: H) -> Self
    where
        H: Fn(WaitPayload) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        Self {
            channel,
            user,
            handler: Arc::new(Box::new(handler)),
            wait_type: WaitType::Message,
            filter: None,
            message: None,
        }
    }

    pub fn reaction<H>(channel: ChannelId, user: UserId, handler: H) -> Self
    where
        H: Fn(WaitPayload) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        Self {
            channel,
            user,
            handler: Arc::new(Box::new(handler)),
            wait_type: WaitType::Reaction,
            filter: None,
            message: None,
        }
    }

    pub async fn send_explanation(mut self, text: &str, http: impl AsRef<Http>) -> Self {
        let res = self
            .channel
            .send_message(http, |m: &mut CreateMessage| m.content(text))
            .await;
        if let Ok(m) = res {
            self.message = Some(m)
        }
        self
    }

    pub fn set_filter<F>(&mut self, filter: F)
    where
        F: Fn(WaitPayload) -> BoxFuture<'static, bool> + Send + Sync + 'static,
    {
        self.filter = Some(Arc::new(Box::new(filter)));
    }

    pub async fn listen(self, data: Arc<RwLock<TypeMap>>) {
        if let Some(waiting) = data.write().await.get_mut::<WaitForKey>() {
            waiting.insert((self.channel, self.user), self);
        }
    }

    async fn handle(&self, payload: WaitPayload) -> bool {
        let run = match payload {
            WaitPayload::Message(_) if self.wait_type == WaitType::Message => true,
            WaitPayload::Reaction(_) if self.wait_type == WaitType::Reaction => true,
            _ => false,
        };

        if run {
            match &self.filter {
                Some(filter) if filter(payload.clone()).await => {
                    (self.handler)(payload).await;
                    true
                }
                None => {
                    (self.handler)(payload).await;
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }
}

pub async fn waitfor_message(ctx: &Context, message: Message) {
    let filter = (message.channel_id, message.author.id);
    handle_waitfor(ctx, filter, WaitPayload::Message(message)).await
}

pub async fn waitfor_reaction(ctx: &Context, reaction: Reaction) {
    if let Some(user_id) = reaction.user_id {
        let filter = (reaction.channel_id, user_id);
        handle_waitfor(ctx, filter, WaitPayload::Reaction(reaction)).await
    }
}

async fn handle_waitfor(ctx: &Context, filter: (ChannelId, UserId), payload: WaitPayload) {
    let waiter = if let Some(waiting) = ctx.data.read().await.get::<WaitForKey>() {
        if let Some(waiter) = waiting.get(&filter) {
            waiter.clone()
        } else {
            return;
        }
    } else {
        return;
    };

    if waiter.handle(payload.clone()).await {
        if let Some(waiting) = ctx.data.write().await.get_mut::<WaitForKey>() {
            waiting.remove(&filter);
        }
        if let Some(message) = waiter.message {
            let _ = message.delete(ctx).await;
        }
        let _ = payload.delete(ctx);
    }
}

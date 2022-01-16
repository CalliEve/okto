use std::sync::Arc;

use futures::future::BoxFuture;
use serenity::{
    builder::{CreateEmbed, EditInteractionResponse, CreateComponents, CreateActionRow, CreateButton},
    cache::Cache,
    http::Http,
    model::{
        channel::ReactionType,
        id::{
            MessageId, UserId,
        }, interactions::{application_command::ApplicationCommandInteraction, message_component::ButtonStyle, Interaction},
    },
    prelude::{
        Context,
        RwLock,
        TypeMap,
    },
};

use crate::models::caches::EmbedSessionsKey;

type Handler = dyn Fn() -> BoxFuture<'static, ()> + Send + Sync;

#[derive(Debug, Clone)]
pub struct ButtonType {
    pub style: ButtonStyle,
    pub label: String,
    pub emoji: Option<ReactionType>,
}

#[derive(Clone)]
pub struct StatefulOption {
    pub button: ButtonType,
    pub handler: Arc<Handler>,
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
        button: &ButtonType,
        handler: F,
    ) where
        F: Fn() -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        let full_name = if let Some(e) = button.emoji { format!("{} {}", e, name)} else { name.to_owned()};
        self.inner
            .field(full_name, value, inline);
        self.options
            .push(StatefulOption {
                button: button.clone(),
                handler: Arc::new(Box::new(handler)),
            })
    }

    pub fn add_option<F>(&mut self, button: &ButtonType, handler: F)
    where
        F: Fn() -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        self.options
            .push(StatefulOption {
                button: button.clone(),
                handler: Arc::new(Box::new(handler)),
            })
    }

    fn get_components(&self) -> CreateComponents {
        let mut components = CreateComponents::default();

        components.create_action_row(|r: &mut CreateActionRow| {
            for option in &self.options {
            r.create_button(|b: &mut CreateButton| {
                b.style(option.button.style).label(option.button.label.to_string()).custom_id(option.button.label.to_string());

                if let Some(e) = &option.button.emoji {
                    b.emoji(e.to_owned());
                }

                b
            });
            }
            r
        });

        components
    }

    pub async fn show(&self) -> serenity::Result<()> {
        {
            let mut session = self
                .session
                .write()
                .await;
            let http = session
                .http
                .clone();
            session.set_embed(self.clone());

            session.interaction.edit_original_interaction_response(&http, |e: &mut EditInteractionResponse| {e.components(|c: &mut CreateComponents| {
                *c = self.get_components();

                c
            }).create_embed(|e: &mut CreateEmbed| {
                            e.0 = self
                                .inner
                                .0
                                .clone();
                            e
                        })}).await?;

            let msg = session.interaction.get_interaction_response(&http).await?;

            if let Some(embeds) = session
                    .data
                    .write()
                    .await
                    .get_mut::<EmbedSessionsKey>()
                {
                    embeds.insert(
                        msg.id,
                        self.session
                            .clone(),
                    );
                };
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct EmbedSession {
    pub current_state: Option<StatefulEmbed>,
    pub interaction: ApplicationCommandInteraction,
    pub http: Arc<Http>,
    pub data: Arc<RwLock<TypeMap>>,
    pub cache: Arc<Cache>,
    pub author: UserId
}

impl EmbedSession {
    pub fn new(ctx: &Context, interaction: ApplicationCommandInteraction) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            http: ctx
                .http
                .clone(),
            current_state: None,
            interaction,
            data: ctx
                .data
                .clone(),
            cache: ctx
                .cache
                .clone(),
            author: interaction.user.id
        }))
    }

    fn set_embed(&mut self, em: StatefulEmbed) {
        self.current_state = Some(em)
    }
}

pub async fn on_button_click(ctx: &Context, full_interaction: &Interaction) {
    if let Interaction::MessageComponent(interaction) = full_interaction {

        let handler = if let Some(cache) = ctx
            .data
            .read()
            .await
            .get::<EmbedSessionsKey>()
        {
            if let Some(session_lock) = cache.get(&interaction.message.id()) {
                let session = session_lock
                    .read()
                    .await;
                if session.author != interaction.user.id {
                    return;
                }

                session
                    .current_state
                    .as_ref()
                    .and_then(|embed| {
                        let mut handler: Option<Arc<Handler>> = None;

                        for opt in &embed.options {
                            if opt.button.label == interaction.data.custom_id {
                                handler = Some(
                                    opt.handler
                                        .clone(),
                                );
                                break;
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
    if let Some(cache) = ctx
        .data
        .write()
        .await
        .get_mut::<EmbedSessionsKey>()
    {
        cache.remove(&message_id);
    }
}

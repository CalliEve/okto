use std::sync::Arc;

use futures::future::BoxFuture;
use itertools::Itertools;
use serenity::{
    builder::{
        CreateActionRow,
        CreateButton,
        CreateEmbed,
        CreateInteractionResponse,
        CreateInteractionResponseMessage,
        EditInteractionResponse,
    },
    cache::Cache,
    http::Http,
    model::{
        application::{
            ButtonStyle,
            CommandInteraction,
            ComponentInteraction,
            Interaction,
        },
        channel::ReactionType,
        id::{
            MessageId,
            UserId,
        },
    },
    prelude::{
        Context,
        RwLock,
        TypeMap,
    },
    Result,
};

use crate::{
    models::caches::EmbedSessionsKey,
    utils::error_log,
};

type Handler = dyn Fn(ComponentInteraction) -> BoxFuture<'static, ()> + Send + Sync;

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
    pub is_update: bool,
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

    pub fn new_with_embed(session: Arc<RwLock<EmbedSession>>, em: CreateEmbed) -> Self {
        Self {
            inner: em,
            session,
            options: Vec::new(),
        }
    }

    pub fn add_field<F>(
        mut self,
        name: &str,
        value: &str,
        inline: bool,
        button: &ButtonType,
        handler: F,
    ) -> Self
    where
        F: Fn(ComponentInteraction) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        let full_name = if let Some(e) = &button.emoji {
            format!("{e} {name}")
        } else {
            name.to_owned()
        };
        self.inner = self
            .inner
            .field(full_name, value, inline);
        self.options
            .push(StatefulOption {
                button: button.clone(),
                handler: Arc::new(Box::new(handler)),
                is_update: true,
            });

        self
    }

    pub fn add_option<F>(&mut self, button: &ButtonType, handler: F) -> &mut Self
    where
        F: Fn(ComponentInteraction) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        self.options
            .push(StatefulOption {
                button: button.clone(),
                handler: Arc::new(Box::new(handler)),
                is_update: true,
            });

        self
    }

    pub fn add_non_update_option<F>(&mut self, button: &ButtonType, handler: F) -> &mut Self
    where
        F: Fn(ComponentInteraction) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        self.options
            .push(StatefulOption {
                button: button.clone(),
                handler: Arc::new(Box::new(handler)),
                is_update: false,
            });

        self
    }

    fn get_components(&self) -> Vec<CreateActionRow> {
        let mut components = Vec::new();

        for option_batch in &self
            .options
            .iter()
            .chunks(5)
        {
            let mut row = Vec::new();
            for option in option_batch {
                let mut button = CreateButton::new(
                    option
                        .button
                        .label
                        .to_string(),
                )
                .style(
                    option
                        .button
                        .style,
                )
                .label(
                    option
                        .button
                        .label
                        .to_string(),
                );

                if let Some(e) = &option
                    .button
                    .emoji
                {
                    button = button.emoji(e.clone());
                }

                row.push(button);
            }
            components.push(CreateActionRow::Buttons(row))
        }

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

            session
                .interaction
                .edit_response(
                    &http,
                    EditInteractionResponse::new()
                        .components(self.get_components())
                        .content("")
                        .embed(
                            self.inner
                                .clone(),
                        ),
                )
                .await?;

            let msg = session
                .interaction
                .get_response(&http)
                .await?;

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
    pub interaction: CommandInteraction,
    pub http: Arc<Http>,
    pub data: Arc<RwLock<TypeMap>>,
    pub cache: Arc<Cache>,
    pub author: UserId,
}

impl EmbedSession {
    pub async fn new(
        ctx: &Context,
        interaction: CommandInteraction,
        ephemeral: bool,
    ) -> Result<Arc<RwLock<Self>>> {
        interaction
            .create_response(
                &ctx.http,
                if ephemeral {
                    CreateInteractionResponse::Defer(
                        CreateInteractionResponseMessage::new().ephemeral(true),
                    )
                } else {
                    CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new())
                },
            )
            .await?;

        Ok(Arc::new(RwLock::new(Self {
            http: ctx
                .http
                .clone(),
            current_state: None,
            author: interaction
                .user
                .id,
            interaction,
            data: ctx
                .data
                .clone(),
            cache: ctx
                .cache
                .clone(),
        })))
    }

    fn set_embed(&mut self, em: StatefulEmbed) {
        self.current_state = Some(em)
    }
}

pub async fn on_button_click(ctx: &Context, full_interaction: &Interaction) {
    if let Interaction::Component(interaction) = full_interaction {
        let handler = {
            let embed_session = ctx
                .data
                .read()
                .await;

            if let Some(cache) = embed_session.get::<EmbedSessionsKey>() {
                if let Some(session_lock) = cache.get(
                    &interaction
                        .message
                        .id,
                ) {
                    let session = session_lock
                        .read()
                        .await;
                    if session.author
                        != interaction
                            .user
                            .id
                    {
                        return;
                    }

                    session
                        .current_state
                        .as_ref()
                        .and_then(|embed| {
                            let mut handler: Option<(Arc<Handler>, bool)> = None;

                            for opt in &embed.options {
                                if opt
                                    .button
                                    .label
                                    == interaction
                                        .data
                                        .custom_id
                                {
                                    handler = Some((
                                        opt.handler
                                            .clone(),
                                        opt.is_update,
                                    ));
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
            }
        };

        if let Some((handler, true)) = handler {
            let r = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Acknowledge,
                )
                .await;

            if let Err(e) = r {
                error_log(
                    &ctx.http,
                    format!("Got error when responding to interaction: {e}",),
                )
                .await;
            } else {
                handler(interaction.clone()).await;
            }
        } else if let Some((handler, false)) = handler {
            handler(interaction.clone()).await;
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

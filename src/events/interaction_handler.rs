use std::sync::Arc;

use futures::{
    future::BoxFuture,
    stream::{
        self,
        StreamExt,
    },
};
use serenity::{
    client::Context,
    http::Http,
    model::{
        application::{
            component::ComponentType,
            interaction::{
                Interaction,
                InteractionType,
            },
        },
        id::{
            ChannelId,
            UserId,
        },
    },
    Error,
    Result,
};

use crate::{
    models::caches::InteractionKey,
    utils::interaction_builder::InteractionResponseBuilder,
};

type Handler = Arc<Box<dyn Fn(Interaction) -> BoxFuture<'static, ()> + Send + Sync>>;

type Filter = Arc<Box<dyn Fn(Interaction) -> BoxFuture<'static, bool> + Send + Sync>>;

#[derive(Clone)]
pub struct InteractionHandler {
    channel: Option<ChannelId>,
    user: Option<UserId>,
    custom_id: Option<String>,
    interaction_type: Option<InteractionType>,
    component_type: Option<ComponentType>,
    handler: Handler,
    filter: Option<Filter>,
}

impl InteractionHandler {
    pub fn builder<F>(handler: F) -> InteractionHandlerBuilder
    where
        F: Fn(Interaction) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        InteractionHandlerBuilder::new(handler)
    }

    async fn handle(&self, interaction: &Interaction) -> bool {
        println!("handle does get called");
        if self
            .interaction_type
            .filter(|t| *t == interaction.kind())
            .is_some()
        {
            return false;
        }

        match interaction {
            Interaction::MessageComponent(component) => {
                dbg!(component);
                if let Some(user) = self.user {
                    if component
                        .user
                        .id
                        != user
                    {
                        return false;
                    }
                }

                if let Some(channel) = self.channel {
                    if component.channel_id != channel {
                        return false;
                    }
                }

                if let Some(component_type) = self.component_type {
                    if component_type
                        != component
                            .data
                            .component_type
                    {
                        return false;
                    }
                }

                if let Some(custom_id) = &self.custom_id {
                    if !component
                        .data
                        .custom_id
                        .starts_with(custom_id)
                    {
                        return false;
                    }
                }
            },
            Interaction::ModalSubmit(modal) => {
                if let Some(user) = self.user {
                    if modal
                        .user
                        .id
                        != user
                    {
                        return false;
                    }
                }

                if let Some(channel) = self.channel {
                    if modal.channel_id != channel {
                        return false;
                    }
                }
            },
            _ => return false,
        }

        if let Some(component_type) = self.component_type {
            if let Interaction::MessageComponent(component) = &interaction {
                if component_type
                    != component
                        .data
                        .component_type
                {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(filter) = &self.filter {
            if !filter(interaction.clone()).await {
                return false;
            }
        }

        (self.handler)(interaction.clone()).await;

        true
    }
}

#[derive(Clone)]
pub struct InteractionHandlerBuilder {
    inner: InteractionHandler,
}

impl InteractionHandlerBuilder {
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(Interaction) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        Self {
            inner: InteractionHandler {
                handler: Arc::new(Box::new(handler)),
                channel: None,
                user: None,
                custom_id: None,
                interaction_type: None,
                component_type: None,
                filter: None,
            },
        }
    }

    #[allow(dead_code)]
    pub fn set_channel(mut self, channel: ChannelId) -> Self {
        self.inner
            .channel = Some(channel);
        self
    }

    #[allow(dead_code)]
    pub fn set_user(mut self, user: UserId) -> Self {
        self.inner
            .user = Some(user);
        self
    }

    pub fn set_custom_id(mut self, custom_id: String) -> Self {
        self.inner
            .custom_id = Some(custom_id);
        self
    }

    #[allow(dead_code)]
    pub fn set_interaction_type(mut self, interaction_type: InteractionType) -> Self {
        self.inner
            .interaction_type = Some(interaction_type);
        self
    }

    pub fn set_component_type(mut self, component_type: ComponentType) -> Self {
        self.inner
            .component_type = Some(component_type);

        if self
            .inner
            .interaction_type
            .is_none()
        {
            self.inner
                .interaction_type = Some(InteractionType::MessageComponent)
        }

        self
    }

    #[allow(dead_code)]
    pub fn set_filter(mut self, filter: Filter) -> Self {
        self.inner
            .filter = Some(filter);
        self
    }

    pub fn build(self) -> Result<InteractionHandler> {
        if self
            .inner
            .component_type
            .is_some()
        {
            if let Some(interaction_type) = self
                .inner
                .interaction_type
            {
                if interaction_type != InteractionType::MessageComponent {
                    return Err(Error::Other("If the component type has been set, the interaction type must be MessageComponent"));
                }
            }
        }

        Ok(self.inner)
    }
}

pub async fn handle_interaction(ctx: &Context, interaction: &Interaction) {
    if let Some(mut waiting) = ctx
        .data
        .write()
        .await
        .get_mut::<InteractionKey>()
    {
        dbg!(waiting
            .0
            .len());
        waiting.0 = stream::iter(
            waiting
                .0
                .iter(),
        )
        .filter(|h| async {
            !h.handle(interaction)
                .await
        })
        .collect::<Vec<&InteractionHandler>>()
        .await
        .into_iter()
        .cloned()
        .collect::<Vec<InteractionHandler>>();
    }
}

pub async fn respond_to_interaction(
    http: impl AsRef<Http>,
    interaction: &Interaction,
    resp: InteractionResponseBuilder,
    update: bool,
) {
    if update {
        match interaction {
            Interaction::MessageComponent(comp) => {
                comp.edit_original_interaction_response(http, |i| {
                    *i = resp.into();
                    i
                })
                .await
            },
            Interaction::ModalSubmit(modal) => {
                modal
                    .edit_original_interaction_response(http, |i| {
                        *i = resp.into();
                        i
                    })
                    .await
            },
            Interaction::ApplicationCommand(cmd) => {
                cmd.edit_original_interaction_response(http, |i| {
                    *i = resp.into();
                    i
                })
                .await
            },
            _ => panic!("Unsupported interaction for sending a response to"),
        }
        .map(|_| ())
    } else {
        match interaction {
            Interaction::MessageComponent(comp) => {
                comp.create_interaction_response(http, |i| {
                    *i = resp.into();
                    i
                })
                .await
            },
            Interaction::ModalSubmit(modal) => {
                modal
                    .create_interaction_response(http, |i| {
                        *i = resp.into();
                        i
                    })
                    .await
            },
            Interaction::ApplicationCommand(cmd) => {
                cmd.create_interaction_response(http, |i| {
                    *i = resp.into();
                    i
                })
                .await
            },
            _ => panic!("Unsupported interaction for sending a response to"),
        }
        .map(|_| ())
    }
    .expect("Interaction response failed");
}

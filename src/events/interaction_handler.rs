use std::sync::Arc;

use futures::{
    future::BoxFuture,
    stream::{
        self,
        StreamExt,
    },
};
use serenity::{
    all::ComponentInteractionDataKind,
    client::Context,
    http::Http,
    model::{
        application::{
            ComponentType,
            Interaction,
            InteractionType,
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
        if self
            .interaction_type
            .is_none()
            || self
                .interaction_type
                .filter(|t| *t == interaction.kind())
                .is_none()
        {
            return false;
        }

        match interaction {
            Interaction::Component(component) => {
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
                    if !component_type_equals_kind(
                        component_type,
                        &component
                            .data
                            .kind,
                    ) {
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
            Interaction::Modal(modal) => {
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

        if let Some(filter) = &self.filter {
            if !filter(interaction.clone()).await {
                return false;
            }
        }

        (self.handler)(interaction.clone()).await;

        true
    }
}

impl PartialEq for InteractionHandler {
    fn eq(&self, other: &Self) -> bool {
        self.channel == other.channel
            && self.user == other.user
            && self.custom_id == other.custom_id
            && self.interaction_type == other.interaction_type
            && self.component_type == other.component_type
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
                .interaction_type = Some(InteractionType::Component)
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
                if interaction_type != InteractionType::Component {
                    return Err(Error::Other("If the component type has been set, the interaction type must be MessageComponent"));
                }
            }
        }

        Ok(self.inner)
    }
}

pub async fn handle_interaction(ctx: &Context, interaction: &Interaction) {
    let all = {
        if let Some(waiting) = ctx
            .data
            .read()
            .await
            .get::<InteractionKey>()
        {
            waiting
                .0
                .clone()
        } else {
            eprintln!("No waiting interaction cache");
            return;
        }
    };

    let handled = stream::iter(all.iter())
        .filter(|h| {
            async {
                h.handle(interaction)
                    .await
            }
        })
        .collect::<Vec<&InteractionHandler>>()
        .await
        .into_iter()
        .cloned()
        .collect::<Vec<InteractionHandler>>();

    if let Some(waiting) = ctx
        .data
        .write()
        .await
        .get_mut::<InteractionKey>()
    {
        waiting.0 = waiting
            .0
            .iter()
            .filter(|h| !handled.contains(h))
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
            Interaction::Component(comp) => {
                comp.edit_response(http.as_ref(), resp.into())
                    .await
            },
            Interaction::Command(cmd) => {
                cmd.edit_response(http.as_ref(), resp.into())
                    .await
            },
            _ => panic!("Unsupported interaction for sending an update response to"),
        }
        .map(|_| ())
    } else {
        match interaction {
            Interaction::Component(comp) => {
                comp.create_response(
                    http.as_ref(),
                    resp.try_into()
                        .expect("created invalid interaction response"),
                )
                .await
            },
            Interaction::Modal(modal) => {
                modal
                    .create_response(
                        http.as_ref(),
                        resp.try_into()
                            .expect("created invalid interaction response"),
                    )
                    .await
            },
            Interaction::Command(cmd) => {
                cmd.create_response(
                    http.as_ref(),
                    resp.try_into()
                        .expect("created invalid interaction response"),
                )
                .await
            },
            _ => panic!("Unsupported interaction for sending a response to"),
        }
    }
    .expect("Interaction response failed");
}

fn component_type_equals_kind(
    component_type: ComponentType,
    component_kind: &ComponentInteractionDataKind,
) -> bool {
    match component_kind {
        ComponentInteractionDataKind::Button => component_type == ComponentType::Button,
        ComponentInteractionDataKind::StringSelect {
            ..
        } => component_type == ComponentType::StringSelect,
        ComponentInteractionDataKind::UserSelect {
            ..
        } => component_type == ComponentType::UserSelect,
        ComponentInteractionDataKind::ChannelSelect {
            ..
        } => component_type == ComponentType::ChannelSelect,
        ComponentInteractionDataKind::RoleSelect {
            ..
        } => component_type == ComponentType::RoleSelect,
        ComponentInteractionDataKind::MentionableSelect {
            ..
        } => component_type == ComponentType::MentionableSelect,
        ComponentInteractionDataKind::Unknown(_) => false,
    }
}

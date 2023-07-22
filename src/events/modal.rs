use std::{
    collections::HashMap,
    sync::Arc,
};

use futures::future::BoxFuture;
use serenity::{
    builder::CreateInputText,
    http::Http,
    model::{
        application::
            interaction::{
                Interaction,
                InteractionType,
                InteractionResponseType,
            },
        id::UserId,
        prelude::component::{
            ActionRowComponent,
            InputTextStyle,
        },
    },
    prelude::{
        RwLock,
        TypeMap,
    },
    Error,
};

use super::interaction_handler::{
    respond_to_interaction,
    InteractionHandler,
};
use crate::{
    models::caches::InteractionKey,
    utils::interaction_builder::InteractionResponseBuilder,
};

type Handler = Arc<Box<dyn Fn(Vec<(String, String)>) -> BoxFuture<'static, ()> + Send + Sync>>;

#[derive(Clone)]
pub struct Modal {
    title: Option<String>,
    user_id: Option<UserId>,
    fields: Vec<Field>,
    custom_id: Option<String>,
    handler: Handler,
}

impl Modal {
    pub async fn listen(
        self,
        http: Arc<Http>,
        interaction: &Interaction,
        data: Arc<RwLock<TypeMap>>,
    ) {
        self.send(&http, interaction)
            .await;

        let handler = self
            .handler
            .clone();

        let http_clone = http.clone();
        let mut interaction_handler = InteractionHandler::builder(move |interaction| {
            let data = interaction
                .modal_submit()
                .expect("Didn't get a modal submit event in modal");

            let values = data
                .data
                .components
                .clone()
                .into_iter()
                .map(|row| {
                    row.components
                        .into_iter()
                        .flat_map(|comp| {
                            match comp {
                                ActionRowComponent::InputText(input) => {
                                    Some((input.custom_id, input.value))
                                },
                                _ => None,
                            }
                        })
                })
                .flatten()
                .collect::<Vec<_>>();

            let http_clone = http_clone.clone();
            let handler_clone = handler.clone();
            Box::pin(async move {
                let _ = data
                    .create_interaction_response(http_clone, |c| {
                        c.kind(InteractionResponseType::DeferredUpdateMessage)
                    })
                    .await;

                handler_clone(values).await
            })
        })
        .set_interaction_type(InteractionType::ModalSubmit);

        if let Some(user_id) = self.user_id {
            interaction_handler = interaction_handler.set_user(user_id);
        }

        if let Some(custom_id) = self.custom_id {
            interaction_handler = interaction_handler.set_custom_id(custom_id);
        }

        if let Some(waiting) = data
            .write()
            .await
            .get_mut::<InteractionKey>()
        {
            waiting
                .0
                .push(
                    interaction_handler
                        .build()
                        .unwrap(),
                );
        }
    }

    async fn send(&self, http: impl AsRef<Http>, interaction: &Interaction) {
        let mut resp = InteractionResponseBuilder::default()
            .kind(InteractionResponseType::Modal)
            .content(
                self.title
                    .clone()
                    .unwrap_or_else(|| "Modal".to_owned()),
            )
            .components(|comps| {
                comps.create_action_row(|row| {
                    for field in &self.fields {
                        row.add_input_text(
                            field
                                .inner
                                .clone(),
                        );
                    }
                    row
                });
                comps
            });

        if let Some(custom_id) = self
            .custom_id
            .clone()
        {
            resp = resp.custom_id(custom_id);
        }

        respond_to_interaction(http, interaction, resp, false).await;
    }

    pub fn builder<F>(handler: F) -> ModalBuilder
    where
        F: Fn(Vec<(String, String)>) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        ModalBuilder::new(handler)
    }
}

#[derive(Clone)]
pub struct ModalBuilder {
    inner: Modal,
}

impl ModalBuilder {
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(Vec<(String, String)>) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        Self {
            inner: Modal {
                handler: Arc::new(Box::new(handler)),
                title: None,
                custom_id: None,
                user_id: None,
                fields: Vec::new(),
            },
        }
    }

    pub fn build(self) -> Result<Modal, Error> {
        if self
            .inner
            .custom_id
            .is_none()
        {
            return Err(Error::Other("a custom_id is required"));
        }

        Ok(self.inner)
    }

    pub fn set_title<T: ToString + ?Sized>(mut self, title: &T) -> Self {
        self.inner
            .title = Some(title.to_string());
        self
    }

    pub fn set_custom_id<T: ToString + ?Sized>(mut self, custom_id: &T) -> Self {
        self.inner
            .custom_id = Some(custom_id.to_string());
        self
    }

    pub fn set_user(mut self, user_id: UserId) -> Self {
        self.inner
            .user_id = Some(user_id);
        self
    }

    pub fn add_field(mut self, field: Field) -> Self {
        self.inner
            .fields
            .push(field);
        self
    }
}

#[derive(Clone)]
pub struct Field {
    inner: CreateInputText,
}

impl Field {
    pub fn new<T: ToString + ?Sized>(custom_id: &T, label: &T) -> Self {
        let mut inner = CreateInputText(HashMap::new());
        inner
            .custom_id(custom_id.to_string())
            .label(label.to_string());

        Self {
            inner,
        }
    }

    pub fn set_style(mut self, style: InputTextStyle) -> Self {
        self.inner
            .style(style);
        self
    }

    pub fn set_required(mut self) -> Self {
        self.inner
            .required(true);
        self
    }

    pub fn set_min_length(mut self, min_length: u64) -> Self {
        self.inner
            .min_length(min_length);
        self
    }

    pub fn set_max_length(mut self, max_length: u64) -> Self {
        self.inner
            .max_length(max_length);
        self
    }

    pub fn set_placeholder<T: ToString + ?Sized>(mut self, placeholder: &T) -> Self {
        self.inner
            .placeholder(placeholder.to_string());
        self
    }
}

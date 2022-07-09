use std::{
    collections::HashMap,
    sync::Arc,
};

use futures::future::BoxFuture;
use serenity::{
    builder::CreateInteractionResponse,
    http::Http,
    model::interactions::{
        message_component::ActionRowComponent,
        Interaction,
        InteractionResponseType,
        InteractionType,
    }, prelude::{RwLock, TypeMap}, Error,
};

use crate::models::caches::InteractionKey;

use super::interaction_handler::InteractionHandler;

type Handler = Arc<Box<dyn Fn(HashMap<String, String>) -> BoxFuture<'static, ()> + Send + Sync>>;

#[derive(Clone)]
pub struct Modal {
    title: Option<String>,
    questions: Vec<Question>,
    custom_id: Option<String>,
    handler: Handler,
}

#[derive(Clone)]
pub struct Question {
    pub custom_id: Option<String>,
    pub label: String,
}

impl Modal {
    pub async fn listen(
        self,
        http: impl AsRef<Http>,
        interaction: &Interaction,
        data: Arc<RwLock<TypeMap>>,
    ) {
        self.send(http, interaction)
            .await;

        let handler = self
            .handler
            .clone();
        let questions = self
            .questions
            .clone();

        let mut interaction_handler = InteractionHandler::builder(move |interaction| {
            let handler = handler.clone();
            let questions = questions.clone();
            Box::pin(async move {
                let data = interaction
                    .clone()
                    .modal_submit()
                    .expect("Didn't get a modal submit in modal");
                let res = data
                    .data
                    .components
                    .clone()
                    .into_iter()
                    .flat_map(|row| {
                        row.components
                            .into_iter()
                    })
                    .map(|comp| {
                        if let ActionRowComponent::InputText(text) = comp {
                            text
                        } else {
                            panic!("Modal contained something else than text")
                        }
                    })
                    .map(|text| (text.custom_id, text.value))
                    .collect::<HashMap<String, String>>();

                handler(res).await;
            })
        })
        .set_interaction_type(InteractionType::ModalSubmit);

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
        let mut resp = CreateInteractionResponse::default();
        resp.kind(InteractionResponseType::Modal)
            .interaction_response_data(|r| {
                r.title(
                    self.title
                        .clone()
                        .unwrap_or_else(|| "Please enter your answers below".to_owned()),
                )
                .components(|comps| {
                    for question in &self.questions {
                        comps.create_action_row(|row| {
                            row.create_input_text(|text| {
                                text.custom_id(
                                    question
                                        .custom_id
                                        .as_ref()
                                        .unwrap_or(&question.label),
                                )
                                .label(&question.label)
                            })
                        });
                    }
                    comps
                });

                if let Some(custom_id) = self
                    .custom_id
                    .clone()
                {
                    r.custom_id(custom_id);
                }

                r
            });

        match interaction {
            Interaction::MessageComponent(comp) => {
                comp.create_interaction_response(http, |i| {
                    *i = resp;
                    i
                })
                .await
            },
            Interaction::ModalSubmit(modal) => {
                modal
                    .create_interaction_response(http, |i| {
                        *i = resp;
                        i
                    })
                    .await
            },
            Interaction::ApplicationCommand(cmd) => {
                cmd.create_interaction_response(http, |i| {
                    *i = resp;
                    i
                })
                .await
            },
            _ => panic!("Unsupported interaction for sending a select menu to"),
        };
    }

    pub fn builder<F>(handler: F) -> ModalBuilder
    where
        F: Fn(HashMap<String, String>) -> BoxFuture<'static, ()> + Send + Sync + 'static,
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
        F: Fn(HashMap<String, String>) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        Self {
            inner: Modal {
                handler: Arc::new(Box::new(handler)),
                title: None,
                custom_id: None,
                questions: Vec::new(),
            },
        }
    }

    pub fn build(self) -> Result<Modal, Error> {
        if self.inner.questions.is_empty() || self.inner.questions.len() > 5 {
            return Err(Error::Other(
                "Unsupported amount of fields in a modal",
            ));
        }
        
        Ok(self.inner)
    }
    
    pub fn set_custom_id(self, custom_id: impl ToString) -> Self {
        self.inner
            .custom_id = Some(custom_id.to_string());
        self
    }

    pub fn set_questions(self, questions: Vec<Question>) -> Self {
        self.inner
            .questions = questions;
        self
    }
    
    pub fn set_title(self, title: impl ToString) -> Self {
        self.inner
            .title = Some(title.to_string());
        self
    }
}

impl Question {
    pub fn new(label: impl ToString) -> Self {
        Self {
            label: label.to_string(),
            custom_id: None,
        }
    }

    pub fn set_custom_id(self, custom_id: impl ToString) -> Self {
        self.custom_id = Some(custom_id.to_string());
        self
    }
}


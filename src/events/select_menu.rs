use std::{
    collections::HashMap,
    sync::Arc,
};

use futures::future::BoxFuture;
use itertools::Itertools;
use serenity::http::Http;
use serenity::{
    builder::CreateInteractionResponse,
    model::interactions::message_component::ComponentType,
};
use serenity::{
    model::interactions::Interaction,
    prelude::{
        RwLock,
        TypeMap,
    },
    Error,
};

use super::interaction_handler::InteractionHandler;
use crate::models::caches::InteractionKey;

type Handler = Arc<Box<dyn Fn(String) -> BoxFuture<'static, ()> + Send + Sync>>;

#[derive(Clone)]
pub struct SelectMenu {
    description: Option<String>,
    options: HashMap<String, String>,
    ephemeral: bool,
    custom_id: Option<String>,
    handler: Handler,
}

impl SelectMenu {
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
        let options = self
            .options
            .clone();
        let mut interaction_handler = InteractionHandler::builder(move |interaction| {
            let handler = handler.clone();
            let options = options.clone();
            Box::pin(async move {
                let data = interaction
                    .clone()
                    .message_component()
                    .expect("Didn't get a message component in select menu");
                let key = data
                    .data
                    .values
                    .first()
                    .expect("No values returned from select menu");

                let chosen = options
                    .get(key)
                    .expect("Not a valid choice in select menu");

                handler(chosen.clone()).await;
            })
        })
        .set_component_type(ComponentType::SelectMenu);

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
        resp.interaction_response_data(|r| {
            r.ephemeral(self.ephemeral)
                .content(
                    self.description
                        .clone()
                        .unwrap_or_else(|| "Select an option".to_owned()),
                );

            r.components(|comps| {
                for chunk in &self
                    .options
                    .iter()
                    .chunks(25)
                {
                    comps.create_action_row(|row| {
                        row.create_select_menu(|menu| {
                            menu.max_values(1)
                                .options(|options| {
                                    for (key, value) in chunk {
                                        options.create_option(|opt| {
                                            opt.value(key)
                                                .label(value)
                                        });
                                    }
                                    options
                                })
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

    pub fn builder<F>(handler: F) -> SelectMenuBuilder
    where
        F: Fn(String) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        SelectMenuBuilder::new(handler)
    }
}

#[derive(Clone)]
pub struct SelectMenuBuilder {
    inner: SelectMenu,
}

impl SelectMenuBuilder {
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(String) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        Self {
            inner: SelectMenu {
                handler: Arc::new(Box::new(handler)),
                ephemeral: false,
                description: None,
                custom_id: None,
                options: HashMap::new(),
            },
        }
    }

    pub fn build(self) -> Result<SelectMenu, Error> {
        if self
            .inner
            .options
            .len()
            < 1
            || self
                .inner
                .options
                .len()
                > 125
        {
            return Err(Error::Other(
                "Unsupported amount of options in a select menu",
            ));
        }

        Ok(self.inner)
    }

    pub fn set_description(self, description: impl ToString) -> Self {
        self.inner
            .description = Some(description.to_string());
        self
    }

    pub fn set_custom_id(self, custom_id: impl ToString) -> Self {
        self.inner
            .custom_id = Some(custom_id.to_string());
        self
    }

    pub fn set_options(self, options: HashMap<String, String>) -> Self {
        self.inner
            .options = options;
        self
    }

    pub fn make_ephemeral(self) -> Self {
        self.inner
            .ephemeral = true;
        self
    }
}

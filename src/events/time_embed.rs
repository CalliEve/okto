use std::sync::Arc;

use chrono::Duration;
use futures::future::BoxFuture;
use serenity::{
    model::{
        application::component::ButtonStyle,
        channel::ReactionType,
    },
    prelude::RwLock,
};

use super::statefulembed::{
    ButtonType,
    EmbedSession,
    StatefulEmbed,
};
use crate::utils::{
    format_duration,
    StandardButton,
};

type Handler = Arc<Box<dyn Fn(Duration) -> BoxFuture<'static, ()> + Send + Sync>>;

#[derive(Clone)]
pub struct TimeEmbed {
    session: Arc<RwLock<EmbedSession>>,
    handler: Handler,
    duration: Duration,
}

impl TimeEmbed {
    pub fn new<F>(session: Arc<RwLock<EmbedSession>>, handler: F) -> Self
    where
        F: Fn(Duration) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    {
        Self {
            session,
            handler: Arc::new(Box::new(handler)),
            duration: Duration::zero(),
        }
    }

    pub async fn listen(self) {
        self.show_embed()
            .await;
    }

    fn show_embed(self) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            let mut embed = StatefulEmbed::new_with(
                self.session
                    .clone(),
                |em| {
                    em.description(
                        if self.duration > Duration::zero() {
                            format!(
                                "Setting a reminder for {} before the moment of launch.",
                                format_duration(self.duration, false)
                            )
                        } else {
                            "Please start specifying a duration using the buttons below:".to_owned()
                        },
                    )
                },
            );

            let self_1_day = self.clone();
            let self_6_hours = self.clone();
            let self_1_hour = self.clone();
            let self_15_minutes = self.clone();
            let self_5_minute = self.clone();
            let self_close = self.clone();

            embed
                .add_option(&add_button("1 day"), move |_| {
                    self_1_day
                        .add_duration(Duration::days(1))
                        .show_embed()
                })
                .add_option(&add_button("6 hours"), move |_| {
                    self_6_hours
                        .add_duration(Duration::hours(6))
                        .show_embed()
                })
                .add_option(&add_button("1 hour"), move |_| {
                    self_1_hour
                        .add_duration(Duration::hours(1))
                        .show_embed()
                })
                .add_option(&add_button("15 minutes"), move |_| {
                    self_15_minutes
                        .add_duration(Duration::minutes(15))
                        .show_embed()
                })
                .add_option(&add_button("5 minutes"), move |_| {
                    self_5_minute
                        .add_duration(Duration::minutes(5))
                        .show_embed()
                })
                .add_option(
                    &StandardButton::Submit.to_button(),
                    move |_| (self_close.handler)(self_close.duration),
                );

            if let Err(e) = embed
                .show()
                .await
            {
                dbg!(e);
            }
        })
    }

    fn add_duration(&self, duration: Duration) -> Self {
        Self {
            session: self
                .session
                .clone(),
            handler: self
                .handler
                .clone(),
            duration: duration + self.duration,
        }
    }
}

fn add_button(text: &str) -> ButtonType {
    ButtonType {
        label: text.to_owned(),
        style: ButtonStyle::Primary,
        emoji: Some(ReactionType::from('➕')),
    }
}

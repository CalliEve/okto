use std::{
    fmt::{self, Display},
    io::ErrorKind as IoErrorKind,
    sync::Arc,
};

use chrono::{Duration, Utc};
use futures::stream::StreamExt;
use mongodb::{
    bson::{self, doc, document::Document},
    error::{Error as MongoError, ErrorKind as MongoErrorKind, Result as MongoResult},
    options::UpdateOptions,
    Collection, Database,
};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor},
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{
        channel::Message,
        id::{ChannelId, GuildId, RoleId, UserId},
    },
    prelude::{Context, RwLock},
};

use crate::{
    events::{
        statefulembed::{EmbedSession, StatefulEmbed},
        waitfor::{WaitFor, WaitPayload},
    },
    models::{caches::DatabaseKey, reminders::Reminder},
    utils::{
        constants::*,
        format_duration, parse_duration, parse_id,
        reminders::{get_guild_settings, get_user_settings},
        temp_message,
    },
};

#[group]
#[commands(notifychannel, notifyme)]
struct Reminders;

#[command]
#[only_in(guild)]
#[required_permissions(MANAGE_GUILD)]
#[usage("Run with channel mention as argument to have the bot post reminders to that channel, defaults to current channel")]
#[description("Manage the reminders and notifications posted by the bot in this server")]
async fn notifychannel(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if msg.guild_id.is_none() {
        return Ok(());
    }

    let target_channel =
        if let Some(channel_id) = args.current().and_then(|c| parse_id(c)).map(ChannelId) {
            channel_id
                .to_channel_cached(&ctx)
                .await
                .map_or(msg.channel_id, |channel| channel.id())
        } else {
            msg.channel_id
        };

    let ses = EmbedSession::new(&ctx, msg.channel_id, msg.author.id);

    main_menu(ses, ID::Channel((target_channel, msg.guild_id.unwrap()))).await;

    Ok(())
}

#[command]
#[description("Setup reminders and notifications from the bot in your DMs")]
async fn notifyme(ctx: &Context, msg: &Message) -> CommandResult {
    let dm = if msg.guild_id.is_some() {
        msg.author.create_dm_channel(&ctx).await?.id
    } else {
        msg.channel_id
    };

    let ses = EmbedSession::new(&ctx, dm, msg.author.id);

    main_menu(ses, ID::User(msg.author.id)).await;

    Ok(())
}

// ---- pages functions ----

fn main_menu(ses: Arc<RwLock<EmbedSession>>, id: ID) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(&Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Reminder Settings").icon_url(DEFAULT_ICON)
                })
        });

        let reminder_ses = ses.clone();
        em.add_field(
            "Reminders",
            "Set at which times you want to get launch reminders",
            false,
            &'⏰'.into(),
            move || {
                let reminder_ses = reminder_ses.clone();
                Box::pin(async move { reminders_page(reminder_ses.clone(), id).await })
            },
        );

        let filters_ses = ses.clone();
        em.add_field(
            "Filters",
            "Set which agencies to filter out of launch reminders, making you not get any reminders for these agencies again",
            false,
            &'📝'.into(),
            move || {
                let filters_ses = filters_ses.clone();
                Box::pin(async move { filters_page(filters_ses.clone(), id).await })
            },
        );

        let allow_filters_ses = ses.clone();
        em.add_field(
            "Allow Filters",
            "Set which agencies to filter launch reminders for, making you get only reminders for these agencies",
            false,
            &'🔍'.into(),
            move || {
                let allow_filters_ses = allow_filters_ses.clone();
                Box::pin(async move { allow_filters_page(allow_filters_ses.clone(), id).await })
            },
        );

        if id.guild_specific() {
            let mention_ses = ses.clone();
            em.add_field(
                "Mentions",
                "Set which roles should be mentioned when posting reminders",
                false,
                &'🔔'.into(),
                move || {
                    let mention_ses = mention_ses.clone();
                    Box::pin(async move { mentions_page(mention_ses.clone(), id).await })
                },
            );
        }

        let other_ses = ses.clone();
        em.add_field(
            "Other",
            "Enable other notifications",
            false,
            &'🛎'.into(),
            move || {
                let other_ses = other_ses.clone();
                Box::pin(async move { other_page(other_ses.clone(), id).await })
            },
        );

        let close_ses = ses.clone();
        em.add_field(
            "Close",
            "Close this menu",
            false,
            &'❌'.into(),
            move || {
                let close_ses = close_ses.clone();
                Box::pin(async move {
                    let lock = close_ses.read().await;
                    if let Some(m) = lock.message.as_ref() {
                        let _ = m.delete(&lock.http).await;
                    };
                })
            },
        );

        let result = em.show().await;
        if result.is_err() {
            dbg!(result.unwrap_err());
        }
    })
}

fn reminders_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let reminders_res = get_reminders(&ses, id).await;
        let description = match reminders_res {
            Ok(reminders) if !reminders.is_empty() => {
                let mut text = "The following reminders have been set:".to_owned();
                for reminder in &reminders {
                    text.push_str(&format!(
                        "\n- {}",
                        format_duration(reminder.get_duration(), false)
                    ))
                }
                text
            }
            _ => "No reminders have been set yet".to_owned(),
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(&Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Reminders").icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let add_ses = ses.clone();
        em.add_field(
        "Add Reminder",
        "Add a new reminder",
        false,
        &PROGRADE,
        move || Box::pin({
            let add_ses = add_ses.clone();
            async move {
                let inner_ses = add_ses.clone();
                let channel_id = inner_ses.read().await.channel;
                let user_id = inner_ses.read().await.author;
                let wait_ses = add_ses.clone();

                WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                    let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                        if let WaitPayload::Message(message) = payload {
                            let dur = parse_duration(&message.content);
                            if !dur.is_zero() {
                                add_reminder(&wait_ses.clone(), id, dur).await;
                            }
                            reminders_page(wait_ses.clone(), id).await;
                        }
                    })
                })
                .send_explanation(
                    "Send how long before the launch you want the reminder.\nPlease give the time in the format of `1w 2d 3h 4m`",
                    &inner_ses.read().await.http,
                ).await
                .listen(inner_ses.read().await.data.clone())
                .await;
            }
        }));

        let remove_ses = ses.clone();
        em.add_field(
        "Remove Reminder",
        "Remove a reminder",
        false,
        &RETROGRADE,
        move || {
            let remove_ses = remove_ses.clone();
            Box::pin(async move {
                let inner_ses = remove_ses.clone();
                let channel_id = inner_ses.read().await.channel;
                let user_id = inner_ses.read().await.author;
                let wait_ses = remove_ses.clone();

                WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                    let wait_ses = wait_ses.clone();
                    Box::pin(async move {
                        if let WaitPayload::Message(message) = payload {
                            remove_reminder(&wait_ses.clone(), id, parse_duration(&message.content)).await;
                            reminders_page(wait_ses.clone(), id).await;
                        }
                    })
                })
                .send_explanation(
                    "Send the time of the reminder you want to remove.\nPlease give the time in the format of `1w 2d 3h 4m`",
                    &inner_ses.read().await.http,
                ).await
                .listen(inner_ses.read().await.data.clone())
                .await;
            })
        });

        em.add_field(
            "Back",
            "Go back to main menu",
            false,
            &'❌'.into(),
            move || {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em.show().await;
        if result.is_err() {
            dbg!(result.unwrap_err());
        }
    })
}

fn filters_page(ses: Arc<RwLock<EmbedSession>>, id: ID) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let db = if let Some(db_res) = get_db(&ses).await {
            db_res
        } else {
            return;
        };

        let description = match id {
            ID::Channel(channel_id) => {
                let settings_res = get_guild_settings(&db, channel_id.1.into()).await;
                match settings_res {
                    Ok(settings) if !settings.filters.is_empty() => {
                        let mut text = "The following agency filters have been set:".to_owned();
                        for filter in &settings.filters {
                            text.push_str(&format!("\n`{}`", filter))
                        }
                        text
                    }
                    _ => "No agency filters have been set yet".to_owned(),
                }
            }
            ID::User(user_id) => {
                let settings_res = get_user_settings(&db, user_id.into()).await;
                match settings_res {
                    Ok(settings) if !settings.filters.is_empty() => {
                        let mut text = "The following agency filters have been set:".to_owned();
                        for filter in &settings.filters {
                            text.push_str(&format!("\n`{}`", filter))
                        }
                        text
                    }
                    _ => "No agency filters have been set yet".to_owned(),
                }
            }
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(&Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Agency Filters").icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let add_ses = ses.clone();
        em.add_field(
            "Add Filter",
            "Add a new filter",
            false,
            &PROGRADE,
            move || {
                let add_ses = add_ses.clone();
                Box::pin(async move {
                    let inner_ses = add_ses.clone();
                    let channel_id = inner_ses.read().await.channel;
                    let user_id = inner_ses.read().await.author;
                    let wait_ses = add_ses.clone();

                    WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if let WaitPayload::Message(message) = payload {
                                let content = message.content.to_lowercase();
                                if LAUNCH_AGENCIES.contains_key(content.as_str()) {
                                    add_filter(&wait_ses.clone(), id, content, "filters").await;
                                } else {
                                    temp_message(
                                        wait_ses.read().await.channel,
                                        &wait_ses.read().await.http,
                                        "Sorry, this launch agency does not exist in my records, so it can't be filtered on.",
                                        Duration::seconds(5)
                                    ).await;
                                }
                                filters_page(wait_ses.clone(), id).await
                            }
                        })
                    })
                    .send_explanation(
                        "Send the filter name of the agency you do not want to receive reminders for",
                        &inner_ses.read().await.http,
                    )
                    .await
                    .listen(inner_ses.read().await.data.clone())
                    .await;
                })
            },
        );

        let remove_ses = ses.clone();
        em.add_field(
            "Remove Filter",
            "Remove a filter",
            false,
            &RETROGRADE,
            move || {
                let remove_ses = remove_ses.clone();
                Box::pin(async move {
                    let inner_ses = remove_ses.clone();
                    let channel_id = inner_ses.read().await.channel;
                    let user_id = inner_ses.read().await.author;
                    let wait_ses = remove_ses.clone();

                    WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if let WaitPayload::Message(message) = payload {
                                let content = message.content.to_lowercase();
                                if LAUNCH_AGENCIES.contains_key(content.as_str()) {
                                    remove_filter(&wait_ses.clone(), id, content, "filters").await;
                                } else {
                                    temp_message(
                                        wait_ses.read().await.channel,
                                        &wait_ses.read().await.http,
                                        "Sorry, this launch agency does not exist in my records, so it can't be filtered on.",
                                        Duration::seconds(5)
                                    ).await;
                                }
                                filters_page(wait_ses.clone(), id).await
                            }
                        })
                    })
                    .send_explanation(
                        "Send the filter name of the agency you want to receive reminders for again",
                        &inner_ses.read().await.http,
                    )
                    .await
                    .listen(inner_ses.read().await.data.clone())
                    .await;
                })
            },
        );

        em.add_field(
            "Back",
            "Go back to main menu",
            false,
            &'❌'.into(),
            move || {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em.show().await;
        if result.is_err() {
            dbg!(result.unwrap_err());
        }
    })
}

fn allow_filters_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let db = if let Some(db_res) = get_db(&ses).await {
            db_res
        } else {
            return;
        };

        let description = match id {
            ID::Channel(channel_id) => {
                let settings_res = get_guild_settings(&db, channel_id.1.into()).await;
                match settings_res {
                    Ok(settings) if !settings.allow_filters.is_empty() => {
                        let mut text =
                            "The following agency allow filters have been set:".to_owned();
                        for filter in &settings.allow_filters {
                            text.push_str(&format!("\n`{}`", filter))
                        }
                        text
                    }
                    _ => "No agency allow filters have been set yet".to_owned(),
                }
            }
            ID::User(user_id) => {
                let settings_res = get_user_settings(&db, user_id.into()).await;
                match settings_res {
                    Ok(settings) if !settings.allow_filters.is_empty() => {
                        let mut text =
                            "The following agency allow filters have been set:".to_owned();
                        for filter in &settings.allow_filters {
                            text.push_str(&format!("\n`{}`", filter))
                        }
                        text
                    }
                    _ => "No agency allow filters have been set yet".to_owned(),
                }
            }
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(&Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Agency Allow Filters").icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let add_ses = ses.clone();
        em.add_field(
            "Add Allow Filter",
            "Add a new allow filter",
            false,
            &PROGRADE,
            move || {
                let add_ses = add_ses.clone();
                Box::pin(async move {
                    let inner_ses = add_ses.clone();
                    let channel_id = inner_ses.read().await.channel;
                    let user_id = inner_ses.read().await.author;
                    let wait_ses = add_ses.clone();

                    WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if let WaitPayload::Message(message) = payload {
                                let content = message.content.to_lowercase();
                                if LAUNCH_AGENCIES.contains_key(content.as_str()) {
                                    add_filter(&wait_ses.clone(), id, content, "allow_filters").await;
                                } else {
                                    temp_message(
                                        wait_ses.read().await.channel,
                                        &wait_ses.read().await.http,
                                        "Sorry, this launch agency does not exist in my records, so it can't be filtered on.",
                                        Duration::seconds(5)
                                    ).await;
                                }
                                filters_page(wait_ses.clone(), id).await
                            }
                        })
                    })
                    .send_explanation(
                        "Send the filter name of the agency you specifically want to get reminders for",
                        &inner_ses.read().await.http,
                    )
                    .await
                    .listen(inner_ses.read().await.data.clone())
                    .await;
                })
            },
        );

        let remove_ses = ses.clone();
        em.add_field(
            "Remove Allow Filter",
            "Remove an allow filter",
            false,
            &RETROGRADE,
            move || {
                let remove_ses = remove_ses.clone();
                Box::pin(async move {
                    let inner_ses = remove_ses.clone();
                    let channel_id = inner_ses.read().await.channel;
                    let user_id = inner_ses.read().await.author;
                    let wait_ses = remove_ses.clone();

                    WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if let WaitPayload::Message(message) = payload {
                                let content = message.content.to_lowercase();
                                if LAUNCH_AGENCIES.contains_key(content.as_str()) {
                                    remove_filter(&wait_ses.clone(), id, content, "allow_filters").await;
                                } else {
                                    temp_message(
                                        wait_ses.read().await.channel,
                                        &wait_ses.read().await.http,
                                        "Sorry, this launch agency does not exist in my records, so it can't be filtered on.",
                                        Duration::seconds(5)
                                    ).await;
                                }
                                filters_page(wait_ses.clone(), id).await
                            }
                        })
                    })
                    .send_explanation(
                        "Send the filter name of the agency you do not want to receive reminders for again",
                        &inner_ses.read().await.http,
                    )
                    .await
                    .listen(inner_ses.read().await.data.clone())
                    .await;
                })
            },
        );

        em.add_field(
            "Back",
            "Go back to main menu",
            false,
            &'❌'.into(),
            move || {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em.show().await;
        if result.is_err() {
            dbg!(result.unwrap_err());
        }
    })
}

fn mentions_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let db = if let Some(db_res) = get_db(&ses).await {
            db_res
        } else {
            return;
        };

        let description = match id {
            ID::Channel((_, guild_id)) => {
                let settings_res = get_guild_settings(&db, guild_id.into()).await;
                match settings_res {
                    Ok(settings) if !settings.mentions.is_empty() => {
                        let mut text =
                            "The following roles have been set to be mentioned:".to_owned();
                        for role_id in &settings.mentions {
                            let role_opt =
                                role_id.to_role_cached(ses.read().await.cache.clone()).await;
                            if let Some(role) = role_opt {
                                text.push_str(&format!("\n`{}`", role.name))
                            } else {
                                remove_mention(&ses, id, *role_id).await
                            }
                        }
                        text
                    }
                    _ => "No role mentions have been set yet".to_owned(),
                }
            }
            _ => return,
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(&Utc::now())
                .author(|a: &mut CreateEmbedAuthor| a.name("Role Mentions").icon_url(DEFAULT_ICON))
                .description(description)
        });

        let add_ses = ses.clone();
        em.add_field(
            "Add Mention",
            "Add a new role to mention",
            false,
            &PROGRADE,
            move || {
                let add_ses = add_ses.clone();
                Box::pin(async move {
                    let inner_ses = add_ses.clone();
                    let channel_id = inner_ses.read().await.channel;
                    let user_id = inner_ses.read().await.author;
                    let wait_ses = add_ses.clone();

                    WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if let WaitPayload::Message(message) = payload {
                                if let Some(role_id) = parse_id(&message.content) {
                                    add_mention(&wait_ses.clone(), id, role_id.into()).await;
                                    mentions_page(wait_ses.clone(), id).await;
                                } else {
                                    mentions_page(wait_ses.clone(), id).await;
                                    temp_message(
                                        channel_id,
                                        wait_ses.read().await.http.clone(),
                                        "Sorry, I can't find that role, please try again later",
                                        Duration::seconds(5),
                                    )
                                    .await
                                }
                            }
                        })
                    })
                    .send_explanation(
                        "Mention the role you want to have mentioned during launch reminders",
                        &inner_ses.read().await.http,
                    )
                    .await
                    .listen(inner_ses.read().await.data.clone())
                    .await;
                })
            },
        );

        let remove_ses = ses.clone();
        em.add_field(
        "Remove Mention",
        "Remove a role to mention",
        false,
        &RETROGRADE,
        move || {
            let remove_ses = remove_ses.clone();
            Box::pin(async move {
                let inner_ses = remove_ses.clone();
                let channel_id = inner_ses.read().await.channel;
                let user_id = inner_ses.read().await.author;
                let wait_ses = remove_ses.clone();

                WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                    let wait_ses = wait_ses.clone();
                    Box::pin(async move {
                    if let WaitPayload::Message(message) = payload {
                        if let Some(role_id) = parse_id(&message.content) {
                            remove_mention(&wait_ses.clone(), id, role_id.into()).await;
                            mentions_page(wait_ses.clone(), id).await;
                        } else {
                            mentions_page(wait_ses.clone(), id).await;
                            temp_message(
                                channel_id,
                                wait_ses.read().await.http.clone(),
                                "Sorry, I can't find that role, please try again later",
                                Duration::seconds(5),
                            ).await
                        }
                    }
                })
                })
                .send_explanation(
                    "Mention the role you want to have removed from being mentioned during launch reminders", 
                    &inner_ses.read().await.http
                ).await
                .listen(inner_ses.read().await.data.clone())
                .await;
            })
        },
    );

        em.add_field(
            "Back",
            "Go back to main menu",
            false,
            &'❌'.into(),
            move || {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em.show().await;
        if result.is_err() {
            dbg!(result.unwrap_err());
        }
    })
}

fn other_page(ses: Arc<RwLock<EmbedSession>>, id: ID) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let db = if let Some(db_res) = get_db(&ses).await {
            db_res
        } else {
            return;
        };

        let mut scrub_notifications = State::Off;
        let mut outcome_notifications = State::Off;
        let mut mentions = State::Off;
        let mut description = "".to_owned();

        match id {
            ID::Channel((_, guild_id)) => {
                let settings_res = get_guild_settings(&db, guild_id.into()).await;
                if let Ok(settings) = settings_res {
                    if settings.scrub_notifications {
                        scrub_notifications = State::On;
                    }

                    if settings.outcome_notifications {
                        outcome_notifications = State::On;
                    }

                    if settings.mention_others {
                        mentions = State::On;
                    }

                    if let Some(chan) = settings.notifications_channel {
                        description = format!(
                            "\nScrub and outcome notifications will be posted in: <#{}>",
                            chan
                        );
                    } else {
                        description =
                            "\n**warning:** no notifications channel has been set yet!".to_owned()
                    }
                }
            }
            ID::User(user_id) => {
                let settings_res = get_user_settings(&db, user_id.into()).await;
                if let Ok(settings) = settings_res {
                    if settings.scrub_notifications {
                        scrub_notifications = State::On;
                    }

                    if settings.outcome_notifications {
                        outcome_notifications = State::On;
                    }
                }
            }
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(&Utc::now())
                .author(|a: &mut CreateEmbedAuthor| a.name("Other Options").icon_url(DEFAULT_ICON))
                .description(description)
        });

        let scrub_ses = ses.clone();
        em.add_field(
            "Toggle Scrub Notifications",
            &format!("Toggle scrub notifications on and off\nThese notifications notify you when a launch gets delayed.\nThis is currently **{}**", scrub_notifications),
            false,
            &'🛑'.into(),
            move || {
                let scrub_ses = scrub_ses.clone();
                Box::pin(async move {
                    let scrub_ses = scrub_ses.clone();
                    toggle_setting(&scrub_ses, id, "scrub_notifications", !scrub_notifications.as_ref())
                        .await;
                    other_page(scrub_ses, id).await
                })
            },
        );

        let outcome_ses = ses.clone();
        em.add_field(
            "Toggle Outcome Notifications",
            &format!("Toggle outcome notifications on and off\nThese notifications notify you about the outcome of a launch.\nThis is currently **{}**", outcome_notifications),
            false,
            &'🌍'.into(),
            move || {
                let outcome_ses = outcome_ses.clone();
                Box::pin(async move {
                    let outcome_ses = outcome_ses.clone();
                    toggle_setting(&outcome_ses, id, "outcome_notifications", !outcome_notifications.as_ref())
                        .await;
                    other_page(outcome_ses, id).await
                })
            },
        );

        let mentions_ses = ses.clone();
        em.add_field(
            "Toggle Mentions",
            &format!(
                "Toggle mentions for scrub and outcome notifications.\nThis is currently **{}**",
                mentions
            ),
            false,
            &'🔔'.into(),
            move || {
                let mentions_ses = mentions_ses.clone();
                Box::pin(async move {
                    let mentions_ses = mentions_ses.clone();
                    toggle_setting(&mentions_ses, id, "mention_others", !mentions.as_ref()).await;
                    other_page(mentions_ses, id).await
                })
            },
        );

        if id.guild_specific() {
            let chan_ses = ses.clone();
            em.add_field(
                "Set Notification Channel",
                "Set the channel to receive scrub and outcome notifications in, this can only be one per server",
                false,
                &'📩'.into(),
                move || {
                    let chan_ses = chan_ses.clone();
                    Box::pin(async move {
                        let inner_ses = chan_ses.clone();
                        let channel_id = inner_ses.read().await.channel;
                        let user_id = inner_ses.read().await.author;
                        let wait_ses = chan_ses.clone();

                        WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                            let wait_ses = wait_ses.clone();
                            Box::pin(async move {
                            if let WaitPayload::Message(message) = payload {
                                if let Some(channel_id) = parse_id(&message.content) {
                                    set_notification_channel(&wait_ses.clone(), id, channel_id.into()).await;
                                    other_page(wait_ses.clone(), id).await;
                                } else {
                                    other_page(wait_ses.clone(), id).await;
                                    temp_message(
                                        channel_id,
                                        wait_ses.read().await.http.clone(),
                                        "Sorry, I can't find that channel, please try again later",
                                        Duration::seconds(5),
                                    ).await
                                }
                            }
                        })
                        })
                        .send_explanation(
                            "Mention the channel you want to set as the server's notification channel", 
                            &inner_ses.read().await.http
                        ).await
                        .listen(inner_ses.read().await.data.clone())
                        .await;
                    })
                },
            );
        }

        em.add_field(
            "Back",
            "Go back to main menu",
            false,
            &'❌'.into(),
            move || {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em.show().await;
        if result.is_err() {
            dbg!(result.unwrap_err());
        }
    })
}

// ---- db functions ----

async fn get_reminders(ses: &Arc<RwLock<EmbedSession>>, id: ID) -> MongoResult<Vec<Reminder>> {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return Err(MongoError::from(MongoErrorKind::Io(Arc::new(
            IoErrorKind::NotFound.into(),
        ))));
    };

    match id {
        ID::User(user_id) => Ok(bson::from_bson(
            db.collection::<Document>("reminders")
                .find(doc! { "users": { "$in": [user_id.0 as i64] } }, None).await?
                .collect::<Vec<Result<_, _>>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        )?),
        ID::Channel((channel_id, guild_id)) => Ok(bson::from_bson(
            db.collection::<Document>("reminders")
                .find(
                    doc! { "channels": { "$in": [{ "channel": channel_id.0 as i64, "guild": guild_id.0 as i64 }] } },
                    None,
                ).await?
                .collect::<Vec<Result<_, _>>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        )?),
    }
}

async fn add_reminder(ses: &Arc<RwLock<EmbedSession>>, id: ID, duration: Duration) {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    let collection = db.collection::<Document>("reminders");

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$addToSet": {
                    "users": user_id.0 as i64
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        ID::Channel((channel_id, guild_id)) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$addToSet": {
                    "channels": { "channel": channel_id.0 as i64, "guild": guild_id.0 as i64 }
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn remove_reminder(ses: &Arc<RwLock<EmbedSession>>, id: ID, duration: Duration) {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    let collection = db.collection::<Document>("reminders");

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$pull": {
                    "users": user_id.0 as i64
                }
            },
            None,
        ),
        ID::Channel((channel_id, guild_id)) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$pull": {
                    "channels": { "channel": channel_id.0 as i64, "guild": guild_id.0 as i64 }
                }
            },
            None,
        ),
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn add_filter(ses: &Arc<RwLock<EmbedSession>>, id: ID, filter: String, filter_type: &str) {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    if !LAUNCH_AGENCIES.contains_key(filter.as_str()) {
        panic!("agencies does not contain filter {}", &filter);
    }

    let collection: Collection<Document> = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"user": user_id.0 as i64},
            doc! {
                "$addToSet": {
                    filter_type: filter
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$addToSet": {
                    filter_type: filter
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn remove_filter(ses: &Arc<RwLock<EmbedSession>>, id: ID, filter: String, filter_type: &str) {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    let collection: Collection<Document> = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"user": user_id.0 as i64},
            doc! {
                "$pull": {
                    filter_type: filter
                }
            },
            None,
        ),
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$pull": {
                    filter_type: filter
                }
            },
            None,
        ),
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn toggle_setting(ses: &Arc<RwLock<EmbedSession>>, id: ID, setting: &str, val: bool) {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    let collection: Collection<Document> = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"user": user_id.0 as i64},
            doc! {
                "$set": {
                    setting: val
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$set": {
                    setting: val
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn set_notification_channel(ses: &Arc<RwLock<EmbedSession>>, id: ID, channel: ChannelId) {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    let collection = db.collection::<Document>("guild_settings");

    let result = match id {
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$set": {
                    "notifications_channel": channel.0 as i64
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        _ => return,
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn add_mention(ses: &Arc<RwLock<EmbedSession>>, id: ID, role: RoleId) {
    let guild_id = if let ID::Channel((_, guild_id)) = id {
        guild_id
    } else {
        return;
    };

    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    let result = db
        .collection::<Document>("guild_settings")
        .update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$addToSet": {
                    "mentions": role.0 as i64
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        )
        .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn remove_mention(ses: &Arc<RwLock<EmbedSession>>, id: ID, role: RoleId) {
    let guild_id = if let ID::Channel((_, guild_id)) = id {
        guild_id
    } else {
        return;
    };

    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    let result = db
        .collection::<Document>("guild_settings")
        .update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$pull": {
                    "mentions": role.0 as i64
                }
            },
            None,
        )
        .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

// ---- utils ----

#[derive(Copy, Clone)]
enum ID {
    Channel((ChannelId, GuildId)),
    User(UserId),
}

impl ID {
    fn guild_specific(&self) -> bool {
        matches!(self, Self::Channel(_))
    }
}

async fn get_db(ses: &Arc<RwLock<EmbedSession>>) -> Option<Database> {
    if let Some(db) = ses.read().await.data.read().await.get::<DatabaseKey>() {
        Some(db.clone())
    } else {
        println!("Could not get a database");
        None
    }
}

#[derive(Copy, Clone)]
enum State {
    On,
    Off,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::On => write!(f, "ON"),
            Self::Off => write!(f, "OFF"),
        }
    }
}

impl AsRef<bool> for State {
    fn as_ref(&self) -> &bool {
        match self {
            Self::On => &true,
            Self::Off => &false,
        }
    }
}

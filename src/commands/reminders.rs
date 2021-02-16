use std::{
    io::ErrorKind as IoErrorKind,
    sync::Arc,
};

use chrono::{
    Duration,
    Utc,
};
use futures::stream::StreamExt;
use mongodb::{
    bson::{
        self,
        doc,
    },
    error::{
        Error as MongoError,
        ErrorKind as MongoErrorKind,
        Result as MongoResult,
    },
    options::UpdateOptions,
    Database,
};
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
    },
    framework::standard::{
        macros::{
            command,
            group,
        },
        Args,
        CommandResult,
    },
    model::{
        channel::Message,
        id::{
            ChannelId,
            GuildId,
            RoleId,
            UserId,
        },
    },
    prelude::{
        Context,
        RwLock,
    },
};

use crate::{
    events::{
        statefulembed::{
            EmbedSession,
            StatefulEmbed,
        },
        waitfor::{
            WaitFor,
            WaitPayload,
        },
    },
    models::{
        caches::DatabaseKey,
        reminders::Reminder,
    },
    utils::{
        constants::*,
        format_duration,
        parse_duration,
        parse_id,
        reminders::{
            get_guild_settings,
            get_user_settings,
        },
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
            "Set which agencies to filter out of launch reminders",
            false,
            &'📝'.into(),
            move || {
                let filters_ses = filters_ses.clone();
                Box::pin(async move { filters_page(filters_ses.clone(), id).await })
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
            },
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
                    },
                    _ => "No agency filters have been set yet".to_owned(),
                }
            },
            ID::User(user_id) => {
                let settings_res = get_user_settings(&db, user_id.into()).await;
                match settings_res {
                    Ok(settings) if !settings.filters.is_empty() => {
                        let mut text = "The following agency filters have been set:".to_owned();
                        for filter in &settings.filters {
                            text.push_str(&format!("\n`{}`", filter))
                        }
                        text
                    },
                    _ => "No agency filters have been set yet".to_owned(),
                }
            },
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
                            println!("triggered add filter payload handler");
                            if let WaitPayload::Message(message) = payload {
                                add_filter(&wait_ses.clone(), id, message.content).await;
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
                                remove_filter(&wait_ses.clone(), id, message.content).await;
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
                    },
                    _ => "No role mentions have been set yet".to_owned(),
                }
            },
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

        let mut scrub_notifications = false;

        let description = match id {
            ID::Channel((_, guild_id)) => {
                let settings_res = get_guild_settings(&db, guild_id.into()).await;
                match settings_res {
                    Ok(settings) => {
                        let mut text = "The following options have been enabled:".to_owned();

                        if settings.scrub_notifications {
                            scrub_notifications = settings.scrub_notifications;
                            text.push_str("\nScrub notifications");
                        }

                        if let Some(chan) = settings.notifications_channel {
                            text.push_str(&format!(
                                "\nScrub notifications will be posted in: <#{}>",
                                chan
                            ));
                        } else {
                            text.push_str(
                                "\n**warning:** no notifications channel has been set yet!",
                            )
                        }

                        text
                    },
                    Err(_) => "No settings found".to_owned(),
                }
            },
            ID::User(user_id) => {
                let settings_res = get_user_settings(&db, user_id.into()).await;
                match settings_res {
                    Ok(settings) => {
                        let mut text = "The following options have been enabled:".to_owned();

                        if settings.scrub_notifications {
                            scrub_notifications = settings.scrub_notifications;
                            text.push_str("\nScrub notifications");
                        }

                        text
                    },
                    Err(_) => "No settings have been found".to_owned(),
                }
            },
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
            "Toggle scrub notifications on and off",
            false,
            &'🛑'.into(),
            move || {
                let scrub_ses = scrub_ses.clone();
                Box::pin(async move {
                    let scrub_ses = scrub_ses.clone();
                    toggle_setting(&scrub_ses, id, "scrub_notifications", !scrub_notifications)
                        .await;
                    other_page(scrub_ses, id).await
                })
            },
        );

        if id.guild_specific() {
            let chan_ses = ses.clone();
            em.add_field(
                "Set Notification Channel",
                "Set the channel to receive scrub notifications in, this can only be one per server",
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
        return Err(MongoError::from(MongoErrorKind::Io(
            IoErrorKind::NotFound.into(),
        )));
    };

    match id {
        ID::User(user_id) => Ok(bson::from_bson(
            db.collection("reminders")
                .find(doc! { "users": { "$in": [user_id.0] } }, None).await?
                .collect::<Vec<Result<_, _>>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        )?),
        ID::Channel((channel_id, guild_id)) => Ok(bson::from_bson(
            db.collection("reminders")
                .find(
                    doc! { "channels": { "$in": [{ "channel": channel_id.0, "guild": guild_id.0 }] } },
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

    let collection = db.collection("reminders");

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$addToSet": {
                    "users": user_id.0
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        ID::Channel((channel_id, guild_id)) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$addToSet": {
                    "channels": { "channel": channel_id.0, "guild": guild_id.0 }
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

    let collection = db.collection("reminders");

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$pull": {
                    "users": user_id.0
                }
            },
            None,
        ),
        ID::Channel((channel_id, guild_id)) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$pull": {
                    "channels": { "channel": channel_id.0, "guild": guild_id.0 }
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

async fn add_filter(ses: &Arc<RwLock<EmbedSession>>, id: ID, filter: String) {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    if !LAUNCH_AGENCIES.contains_key(filter.as_str()) {
        println!("agencies does not contain filter {}", &filter);
        return;
    }

    let collection = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"user": user_id.0},
            doc! {
                "$addToSet": {
                    "filters": filter
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0},
            doc! {
                "$addToSet": {
                    "filters": filter
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

async fn remove_filter(ses: &Arc<RwLock<EmbedSession>>, id: ID, filter: String) {
    let db = if let Some(db) = get_db(&ses).await {
        db
    } else {
        return;
    };

    let collection = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"user": user_id.0},
            doc! {
                "$pull": {
                    "filters": filter
                }
            },
            None,
        ),
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0},
            doc! {
                "$pull": {
                    "filters": filter
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

    let collection = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => collection.update_one(
            doc! {"user": user_id.0},
            doc! {
                "$set": {
                    setting: val
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0},
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

    let collection = db.collection("guild_settings");

    let result = match id {
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0},
            doc! {
                "$set": {
                    "notifications_channel": channel.0
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
        .collection("guild_settings")
        .update_one(
            doc! {"guild": guild_id.0},
            doc! {
                "$addToSet": {
                    "mentions": role.0
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
        .collection("guild_settings")
        .update_one(
            doc! {"guild": guild_id.0},
            doc! {
                "$pull": {
                    "mentions": role.0
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

use std::{
    fmt::{
        self,
        Display,
    },
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
        document::Document,
    },
    error::{
        Error as MongoError,
        ErrorKind as MongoErrorKind,
        Result as MongoResult,
    },
    options::UpdateOptions,
    Collection,
    Database,
};
use okto_framework::macros::command;
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
        CreateInteractionResponse,
    },
    framework::standard::CommandResult,
    model::{
        application::{
            component::ButtonStyle,
            interaction::application_command::{
                ApplicationCommandInteraction,
                CommandDataOptionValue,
            },
            interaction::Interaction,
            interaction::MessageFlags,
        },
        channel::ReactionType,
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
        select_menu::SelectMenu,
        statefulembed::{
            ButtonType,
            EmbedSession,
            StatefulEmbed,
        },
        time_embed::TimeEmbed,
    },
    models::{
        caches::DatabaseKey,
        reminders::Reminder,
    },
    utils::{
        constants::*,
        default_embed,
        default_select_menus::{
            channel_select_menu,
            role_select_menu,
        },
        format_duration,
        reminders::{
            get_guild_settings,
            get_user_settings,
        },
        StandardButton,
    },
};

#[command]
#[required_permissions(MANAGE_GUILD)]
#[default_permission(false)]
#[options(
    {
        option_type: Channel,
        name: "target_channel",
        description: "Channel to set reminders for instead of channel this command was ran in"
    }
)]
/// Manage the reminders and notifications posted by the bot in this server
async fn notifychannel(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> CommandResult {
    if interaction
        .guild_id
        .is_none()
    {
        interaction
            .create_interaction_response(&ctx.http, |m: &mut CreateInteractionResponse| {
                m.interaction_response_data(|c| {
                    c.flags(MessageFlags::EPHEMERAL)
                        .embed(|e: &mut CreateEmbed| {
                            default_embed(e, "This command can only be ran in a server.", false)
                        })
                })
            })
            .await?;

        return Ok(());
    }

    let target_channel = if let Some(channel_id) = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "target_channel")
    {
        channel_id
            .resolved
            .clone()
            .and_then(|v| {
                if let CommandDataOptionValue::Channel(c) = v {
                    Some(c.id)
                } else {
                    None
                }
            })
            .ok_or("Invalid argument given")?
            .to_channel_cached(&ctx)
            .map_or(interaction.channel_id, |channel| channel.id())
    } else {
        interaction.channel_id
    };

    let ses = EmbedSession::new(ctx, interaction.clone(), false).await?;

    main_menu(
        ses,
        ID::Channel((
            target_channel,
            interaction
                .guild_id
                .unwrap(),
        )),
    )
    .await;

    Ok(())
}

#[command]
/// Setup reminders and notifications from the bot in your DMs
async fn notifyme(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    let ses = EmbedSession::new(ctx, interaction.clone(), true).await?;

    main_menu(
        ses,
        ID::User(
            interaction
                .user
                .id,
        ),
    )
    .await;

    Ok(())
}

// ---- pages functions ----

fn main_menu(ses: Arc<RwLock<EmbedSession>>, id: ID) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Reminder Settings")
                        .icon_url(DEFAULT_ICON)
                })
        });

        let reminder_ses = ses.clone();
        em.add_field(
            "Reminders",
            "Set at which times you want to get launch reminders",
            false,
            &ButtonType {
                emoji: Some('⏰'.into()),
                style: ButtonStyle::Primary,
                label: "Reminders".to_owned(),
            },
            move |_| {
                let reminder_ses = reminder_ses.clone();
                Box::pin(async move { reminders_page(reminder_ses.clone(), id).await })
            },
        );

        let filters_ses = ses.clone();
        em.add_field(
            "Filters",
            "Set which agencies to filter out of launch reminders, making you not get any reminders for these agencies again",
            false,
            &ButtonType{ emoji: Some('📝'.into()), style: ButtonStyle::Primary, label: "Filters".to_owned()},
            move |_| {
                let filters_ses = filters_ses.clone();
                Box::pin(async move { filters_page(filters_ses.clone(), id).await })
            },
        );

        let allow_filters_ses = ses.clone();
        em.add_field(
            "Allow Filters",
            "Set which agencies to filter launch reminders for, making you get only reminders for these agencies",
            false,
            &ButtonType{ emoji: Some('🔍'.into()), style: ButtonStyle::Primary, label: "Allow Filters".to_owned()},
            move |_| {
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
                &ButtonType {
                    emoji: Some('🔔'.into()),
                    style: ButtonStyle::Primary,
                    label: "Mentions".to_owned(),
                },
                move |_| {
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
            &ButtonType {
                emoji: Some('🛎'.into()),
                style: ButtonStyle::Primary,
                label: "Other".to_owned(),
            },
            move |_| {
                let other_ses = other_ses.clone();
                Box::pin(async move { other_page(other_ses.clone(), id).await })
            },
        );

        let close_ses = ses.clone();
        em.add_field(
            "Close",
            "Close this menu",
            false,
            &StandardButton::Exit.to_button(),
            move |_| {
                let close_ses = close_ses.clone();
                Box::pin(async move {
                    let lock = close_ses
                        .read()
                        .await;
                    let r = lock
                        .interaction
                        .delete_original_interaction_response(&lock.http)
                        .await;
                    if let Err(e) = r {
                        dbg!(e);
                    }
                })
            },
        );

        let result = em
            .show()
            .await;
        if let Err(err) = result {
            dbg!(err);
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
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Reminders")
                        .icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let add_ses = ses.clone();
        em.add_option(
            &ButtonType {
                label: "Add reminder".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(PROGRADE.clone()),
            },
            move |_| {
                Box::pin({
                    let add_ses = add_ses.clone();
                    async move {
                        let inner_ses = add_ses.clone();
                        let wait_ses = add_ses.clone();

                        TimeEmbed::new(inner_ses, move |dur| {
                            let wait_ses = wait_ses.clone();
                            Box::pin(async move {
                                if !dur.is_zero() {
                                    add_reminder(&wait_ses.clone(), id, dur).await;
                                }
                                reminders_page(wait_ses.clone(), id).await;
                            })
                        })
                        .listen()
                        .await;
                    }
                })
            },
        );

        let remove_ses = ses.clone();
        em.add_option(
            &ButtonType {
                label: "Remove reminder".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(RETROGRADE.clone()),
            },
            move |_| {
                let remove_ses = remove_ses.clone();
                Box::pin(async move {
                    let inner_ses = remove_ses.clone();
                    let wait_ses = remove_ses.clone();

                    TimeEmbed::new(inner_ses, move |dur| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if !dur.is_zero() {
                                remove_reminder(&wait_ses.clone(), id, dur).await;
                            }
                            reminders_page(wait_ses.clone(), id).await;
                        })
                    })
                    .listen()
                    .await;
                })
            },
        );

        em.add_option(
            &ButtonType {
                label: "Back to main menu".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(BACK_EMOJI)),
            },
            move |_| {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em
            .show()
            .await;
        if let Err(err) = result {
            dbg!(err);
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
                let settings_res = get_guild_settings(
                    &db,
                    channel_id
                        .1
                        .into(),
                )
                .await;
                match settings_res {
                    Ok(settings)
                        if !settings
                            .filters
                            .is_empty() =>
                    {
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
                    Ok(settings)
                        if !settings
                            .filters
                            .is_empty() =>
                    {
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
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Agency Filters")
                        .icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let add_ses = ses.clone();
        em.add_option(
            &ButtonType {
                label: "Add filter".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(PROGRADE.clone()),
            },
            move |button_click| {
                let add_ses = add_ses.clone();
                Box::pin(async move {
                    let inner_ses = add_ses.clone();
                    let wait_ses = add_ses.clone();

                    let (user_id, http, data) = {
                        let s = inner_ses
                            .read()
                            .await;
                        (
                            s.author,
                            s.http
                                .clone(),
                            s.data
                                .clone(),
                        )
                    };

                    SelectMenu::builder(move |(choice, _)| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if LAUNCH_AGENCIES.contains_key(choice.as_str()) {
                                add_filter(&wait_ses.clone(), id, choice, "filters").await;
                            } else {
                                panic!("select menu returned unknown choice")
                            }
                            filters_page(wait_ses.clone(), id).await
                        })
                    })
                    .set_description(
                        "Select the name of the agency you do not want to receive reminders for",
                    )
                    .set_custom_id(&format!("{}-add-filter", user_id))
                    .make_ephemeral()
                    .set_options(
                        LAUNCH_AGENCIES
                            .iter()
                            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                            .collect(),
                    )
                    .build()
                    .unwrap()
                    .listen(http, &Interaction::MessageComponent(button_click), data)
                    .await;
                })
            },
        );

        let remove_ses = ses.clone();
        em.add_option(
            &ButtonType {
                label: "Remove filter".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(RETROGRADE.clone()),
            },
            move |button_click| {
                let remove_ses = remove_ses.clone();
                Box::pin(async move {
                    let inner_ses = remove_ses.clone();
                    let wait_ses = remove_ses.clone();

                    let (user_id, http, data) = {
                        let s = inner_ses
                            .read()
                            .await;
                        (
                            s.author,
                            s.http
                                .clone(),
                            s.data
                                .clone(),
                        )
                    };

                    SelectMenu::builder(move |(choice, _)| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if LAUNCH_AGENCIES.contains_key(choice.as_str()) {
                                remove_filter(&wait_ses.clone(), id, choice, "filters").await;
                            } else {
                                panic!("select menu returned unknown choice")
                            }
                            filters_page(wait_ses.clone(), id).await
                        })
                    })
                    .set_description(
                        "Select the name of the agency you want to receive reminders for again",
                    )
                    .set_custom_id(&format!("{}-remove-filter", user_id))
                    .make_ephemeral()
                    .set_options(
                        LAUNCH_AGENCIES
                            .iter()
                            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                            .collect(),
                    )
                    .build()
                    .unwrap()
                    .listen(http, &Interaction::MessageComponent(button_click), data)
                    .await;
                })
            },
        );

        em.add_option(
            &ButtonType {
                label: "Back to main menu".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(BACK_EMOJI)),
            },
            move |_| {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em
            .show()
            .await;
        if let Err(err) = result {
            dbg!(err);
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
                let settings_res = get_guild_settings(
                    &db,
                    channel_id
                        .1
                        .into(),
                )
                .await;
                match settings_res {
                    Ok(settings)
                        if !settings
                            .allow_filters
                            .is_empty() =>
                    {
                        let mut text =
                            "The following agency allow filters have been set:".to_owned();
                        for filter in &settings.allow_filters {
                            text.push_str(&format!("\n`{}`", filter))
                        }
                        text
                    },
                    _ => "No agency allow filters have been set yet".to_owned(),
                }
            },
            ID::User(user_id) => {
                let settings_res = get_user_settings(&db, user_id.into()).await;
                match settings_res {
                    Ok(settings)
                        if !settings
                            .allow_filters
                            .is_empty() =>
                    {
                        let mut text =
                            "The following agency allow filters have been set:".to_owned();
                        for filter in &settings.allow_filters {
                            text.push_str(&format!("\n`{}`", filter))
                        }
                        text
                    },
                    _ => "No agency allow filters have been set yet".to_owned(),
                }
            },
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Agency Allow Filters")
                        .icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let add_ses = ses.clone();
        em.add_option(
            &ButtonType {
                style: ButtonStyle::Primary,
                label: "Add allow filter".to_owned(),
                emoji: Some(PROGRADE.clone()),
            },
            move |button_click| {
                let add_ses = add_ses.clone();
                Box::pin(async move {
                    let inner_ses = add_ses.clone();
                    let wait_ses = add_ses.clone();

                    let (user_id, http, data) = {
                        let s = inner_ses
                            .read()
                            .await;
                        (
                            s.author,
                            s.http
                                .clone(),
                            s.data
                                .clone(),
                        )
                    };

                    SelectMenu::builder(move |(choice, _)| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if LAUNCH_AGENCIES.contains_key(choice.as_str()) {
                                add_filter(&wait_ses.clone(), id, choice, "allow_filters").await;
                            } else {
                                panic!("select menu returned unknown choice")
                            }
                            allow_filters_page(wait_ses.clone(), id).await
                        })
                    })
                    .set_description(
                        "Select the name of the agency you specifically want to get reminders for",
                    )
                    .set_custom_id(&format!("{}-add-allow-filter", user_id))
                    .make_ephemeral()
                    .set_options(
                        LAUNCH_AGENCIES
                            .iter()
                            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                            .collect(),
                    )
                    .build()
                    .unwrap()
                    .listen(http, &Interaction::MessageComponent(button_click), data)
                    .await;
                })
            },
        );

        let remove_ses = ses.clone();
        em.add_option(
            &ButtonType {
                label: "Remove allow filter".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(RETROGRADE.clone())
            },
            move |button_click| {
                let remove_ses = remove_ses.clone();
                Box::pin(async move {
                    let inner_ses = remove_ses.clone();
                    let wait_ses = remove_ses.clone();

                    let (user_id, http, data) = {
                        let s = inner_ses
                                .read()
                                .await;
                        (
                            s.author,
                            s.http
                                .clone(),
                            s.data
                                .clone(),
                        )
                    };

                    SelectMenu::builder(move |(choice, _)| {
                            let wait_ses = wait_ses.clone();
                            Box::pin(async move {
                                if LAUNCH_AGENCIES.contains_key(choice.as_str()) {
                                    remove_filter(&wait_ses.clone(), id, choice, "allow_filters").await;
                                } else {
                                        panic!("select menu returned unknown choice")
                                    }
                                filters_page(wait_ses.clone(), id).await
                            })
                    })
                        .set_description("Select the name of the agency you do not want to receive reminders for again")
                        .set_custom_id(&format!("{}-remove-allow-filter", user_id))
                        .make_ephemeral()
                        .set_options(LAUNCH_AGENCIES.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect())
                        .build()
                        .unwrap()
                        .listen(http, &Interaction::MessageComponent(button_click), data).await;
                })
            },
        );

        em.add_option(
            &ButtonType {
                label: "Back to main menu".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(BACK_EMOJI)),
            },
            move |_| {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em
            .show()
            .await;
        if let Err(err) = result {
            dbg!(err);
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
                    Ok(settings)
                        if !settings
                            .mentions
                            .is_empty() =>
                    {
                        let mut text =
                            "The following roles have been set to be mentioned:".to_owned();
                        for role_id in &settings.mentions {
                            let role_opt = role_id.to_role_cached(
                                ses.read()
                                    .await
                                    .cache
                                    .clone(),
                            );
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
            ID::User(_) => return,
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Role Mentions")
                        .icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let add_ses = ses.clone();
        em.add_option(
            &ButtonType {
                label: "Add mention".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(PROGRADE.clone()),
            },
            move |button_click| {
                let add_ses = add_ses.clone();
                Box::pin(async move {
                    let (user_id, http, data) = {
                        let s = add_ses
                            .read()
                            .await;
                        (
                            s.author,
                            s.http
                                .clone(),
                            s.data
                                .clone(),
                        )
                    };

                    role_select_menu(
                        http,
                        user_id,
                        &Interaction::MessageComponent(button_click),
                        data,
                        move |role_id| {
                            let wait_ses = add_ses.clone();
                            Box::pin(async move {
                                add_mention(&wait_ses.clone(), id, role_id).await;
                                mentions_page(wait_ses.clone(), id).await;
                            })
                        },
                    )
                    .await;
                })
            },
        );

        let remove_ses = ses.clone();
        em.add_option(
            &ButtonType {
                label: "Remove mention".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(RETROGRADE.clone()),
            },
            move |button_click| {
                let remove_ses = remove_ses.clone();
                Box::pin(async move {
                    let (user_id, http, data) = {
                        let s = remove_ses
                            .read()
                            .await;
                        (
                            s.author,
                            s.http
                                .clone(),
                            s.data
                                .clone(),
                        )
                    };

                    role_select_menu(
                        http,
                        user_id,
                        &Interaction::MessageComponent(button_click),
                        data,
                        move |role_id| {
                            let wait_ses = remove_ses.clone();
                            Box::pin(async move {
                                remove_mention(&wait_ses.clone(), id, role_id).await;
                                mentions_page(wait_ses.clone(), id).await;
                            })
                        },
                    )
                    .await;
                })
            },
        );

        em.add_option(
            &ButtonType {
                label: "Back to main menu".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(BACK_EMOJI)),
            },
            move |_| {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em
            .show()
            .await;
        if let Err(err) = result {
            dbg!(err);
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
            },
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
            },
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Other Options")
                        .icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let scrub_ses = ses.clone();
        em.add_field(
            "Toggle Scrub Notifications",
            &format!("Toggle scrub notifications on and off\nThese notifications notify you when a launch gets delayed.\nThis is currently **{}**", scrub_notifications),
            false,
            &ButtonType {
                emoji: Some('🛑'.into()),
                style: ButtonStyle::Primary,
                label: "Toggle Scrubs".to_owned(),
            },
            move |_| {
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
            &ButtonType {
                emoji: Some('🌍'.into()),
                style: ButtonStyle::Primary,
                label: "Toggle Outcomes".to_owned(),
            },
            move |_| {
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
            &ButtonType {
                emoji: Some('🔔'.into()),
                style: ButtonStyle::Primary,
                label: "Toggle Mentions".to_owned(),
            },
            move |_| {
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
                &ButtonType {
                    emoji: Some('📩'.into()),
                    style: ButtonStyle::Primary,
                    label: "Set Notification Channel".to_owned(),
                },
                move |button_click| {
                    let chan_ses = chan_ses.clone();
                    Box::pin(async move {
                    let (user_id, http, data) = {
                        let s = chan_ses
                                .read()
                                .await;
                        (
                            s.author,
                            s.http
                                .clone(),
                            s.data
                                .clone(),
                        )
                    };

                    channel_select_menu(http, user_id, &Interaction::MessageComponent(button_click), data, move |channel_id| {
                        let wait_ses = chan_ses.clone();
                        Box::pin(async move {
                                    set_notification_channel(&wait_ses.clone(), id, channel_id).await;
                                    other_page(wait_ses.clone(), id).await;
                    })}).await;

                    })
                },
            );
        }

        em.add_option(
            &ButtonType {
                label: "Back to main menu".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(BACK_EMOJI)),
            },
            move |_| {
                let ses = ses.clone();
                Box::pin(async move { main_menu(ses.clone(), id).await })
            },
        );

        let result = em
            .show()
            .await;
        if let Err(err) = result {
            dbg!(err);
        }
    })
}

// ---- db functions ----

async fn get_reminders(ses: &Arc<RwLock<EmbedSession>>, id: ID) -> MongoResult<Vec<Reminder>> {
    let db = if let Some(db) = get_db(ses).await {
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
    let db = if let Some(db) = get_db(ses).await {
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
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
        ),
        ID::Channel((channel_id, guild_id)) => collection.update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$addToSet": {
                    "channels": { "channel": channel_id.0 as i64, "guild": guild_id.0 as i64 }
                }
            },
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
        ),
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn remove_reminder(ses: &Arc<RwLock<EmbedSession>>, id: ID, duration: Duration) {
    let db = if let Some(db) = get_db(ses).await {
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
    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    assert!(
        LAUNCH_AGENCIES.contains_key(filter.as_str()),
        "agencies does not contain filter {}",
        &filter
    );

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
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
        ),
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$addToSet": {
                    filter_type: filter
                }
            },
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
        ),
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn remove_filter(ses: &Arc<RwLock<EmbedSession>>, id: ID, filter: String, filter_type: &str) {
    let db = if let Some(db) = get_db(ses).await {
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
    let db = if let Some(db) = get_db(ses).await {
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
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
        ),
        ID::Channel((_, guild_id)) => collection.update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$set": {
                    setting: val
                }
            },
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
        ),
    }
    .await;

    if let Err(e) = result {
        dbg!(e);
    }
}

async fn set_notification_channel(ses: &Arc<RwLock<EmbedSession>>, id: ID, channel: ChannelId) {
    let db = if let Some(db) = get_db(ses).await {
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
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
        ),
        ID::User(_) => return,
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

    let db = if let Some(db) = get_db(ses).await {
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
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
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

    let db = if let Some(db) = get_db(ses).await {
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
    if let Some(db) = ses
        .read()
        .await
        .data
        .read()
        .await
        .get::<DatabaseKey>()
    {
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

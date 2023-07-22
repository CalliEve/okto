use std::{
    fmt::Write,
    sync::Arc,
};

use chrono::Utc;
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
    },
    model::{
        application::{
            component::ButtonStyle,
            interaction::Interaction,
        },
        channel::ReactionType,
        prelude::component::InputTextStyle,
    },
    prelude::RwLock,
};

use super::{
    main_menu,
    settings::{
        add_filter,
        add_mention,
        add_reminder,
        get_reminders,
        remove_filter,
        remove_mention,
        remove_reminder,
        set_notification_channel,
        toggle_setting,
    },
    utils::{
        filter_from_string_input,
        get_db,
        regex_filter_to_string,
        State,
        ID,
    },
};
use crate::{
    events::{
        modal::{
            Field,
            Modal,
        },
        select_menu::SelectMenu,
        statefulembed::{
            ButtonType,
            EmbedSession,
            StatefulEmbed,
        },
        time_embed::TimeEmbed,
    },
    utils::{
        constants::*,
        default_select_menus::{
            channel_select_menu,
            role_select_menu,
        },
        format_duration,
        reminders::{
            get_guild_settings,
            get_user_settings,
        },
    },
};

pub fn reminders_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let reminders_res = get_reminders(&ses, id).await;
        let description = match reminders_res {
            Ok(ref reminders) if !reminders.is_empty() => {
                let mut text = "The following reminders have been set:".to_owned();
                for reminder in reminders {
                    write!(
                        text,
                        "\n- {}",
                        format_duration(reminder.get_duration(), false)
                    )
                    .expect("write to String: can't fail");
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

        if reminders_res.is_ok() {
            let remove_ses = ses.clone();
            em.add_option(
                &ButtonType {
                    label: "Remove reminder".to_owned(),
                    style: ButtonStyle::Primary,
                    emoji: Some(RETROGRADE.clone()),
                },
                move |_| {
                    // TODO: use a dropdown for removal
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

pub fn filters_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Filters")
                        .icon_url(DEFAULT_ICON)
                })
        });

        let filters_ses = ses.clone();
        em.add_field(
            "Filters",
            "Set which agencies to filter out of launch reminders, making you not get any reminders for these agencies again",
            false,
            &ButtonType{ emoji: Some('⛔'.into()), style: ButtonStyle::Primary, label: "Disallow Filters".to_owned()},
            move |_| {
                let filters_ses = filters_ses.clone();
                Box::pin(async move { disallow_filters_page(filters_ses.clone(), id).await })
            },
        );

        let allow_filters_ses = ses.clone();
        em.add_field(
            "Allow Filters",
            "Set which agencies to filter launch reminders for, making you get **only** reminders for these agencies",
            false,
            &ButtonType{ emoji: Some('🔍'.into()), style: ButtonStyle::Primary, label: "Allow Filters".to_owned()},
            move |_| {
                let allow_filters_ses = allow_filters_ses.clone();
                Box::pin(async move { allow_filters_page(allow_filters_ses.clone(), id).await })
            },
        );

        let payload_filters_ses = ses.clone();
        em.add_field(
            "Payload Filters",
            "Add word or regex filters to filter out launches with specific payloads",
            false,
            &ButtonType {
                emoji: Some('📝'.into()),
                style: ButtonStyle::Primary,
                label: "Payload Filters".to_owned(),
            },
            move |_| {
                let allow_filters_ses = payload_filters_ses.clone();
                Box::pin(async move { payload_filters_page(allow_filters_ses.clone(), id).await })
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

fn disallow_filters_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let db = if let Some(db_res) = get_db(&ses).await {
            db_res
        } else {
            return;
        };

        let (description, filters) = match id {
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
                            write!(
                                text,
                                "\n`{}`",
                                LAUNCH_AGENCIES
                                    .get(filter.as_str())
                                    .unwrap_or(&"unknown agency")
                            )
                            .expect("write to String: can't fail");
                        }
                        (
                            text,
                            settings
                                .filters
                                .clone(),
                        )
                    },
                    _ => {
                        (
                            "No agency filters have been set yet".to_owned(),
                            Vec::new(),
                        )
                    },
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
                            write!(
                                text,
                                "\n`{}`",
                                LAUNCH_AGENCIES
                                    .get(filter.as_str())
                                    .unwrap_or(&"unknown agency")
                            )
                            .expect("write to String: can't fail");
                        }
                        (
                            text,
                            settings
                                .filters
                                .clone(),
                        )
                    },
                    _ => {
                        (
                            "No agency filters have been set yet".to_owned(),
                            Vec::new(),
                        )
                    },
                }
            },
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Launch Agency Disallow Filters")
                        .icon_url(DEFAULT_ICON)
                })
                .description(description)
        });

        let add_ses = ses.clone();
        let filters_clone = filters.clone();
        em.add_option(
            &ButtonType {
                label: "Add filter".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(PROGRADE.clone()),
            },
            move |button_click| {
                let add_ses = add_ses.clone();
                let filters_clone = filters_clone.clone();
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
                                eprintln!("select menu returned unknown choice")
                            }
                            disallow_filters_page(wait_ses.clone(), id).await
                        })
                    })
                    .set_description(
                        "Select the name of the agency you do not want to receive reminders for",
                    )
                    .set_custom_id(&format!("{}-add-filter", user_id))
                    .set_user(user_id)
                    .set_options(
                        LAUNCH_AGENCIES
                            .iter()
                            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                            .filter(|(k, _)| !filters_clone.contains(k))
                            .collect(),
                    )
                    .build()
                    .unwrap()
                    .listen(
                        http,
                        &Interaction::MessageComponent(button_click),
                        data,
                    )
                    .await;
                })
            },
        );

        if !filters.is_empty() {
            let remove_ses = ses.clone();
            let filters_clone = filters.clone();
            em.add_option(
                &ButtonType {
                    label: "Remove filter".to_owned(),
                    style: ButtonStyle::Primary,
                    emoji: Some(RETROGRADE.clone()),
                },
                move |button_click| {
                    let remove_ses = remove_ses.clone();
                    let filters_clone = filters_clone.clone();
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
                                    eprintln!("select menu returned unknown choice")
                                }
                                disallow_filters_page(wait_ses.clone(), id).await
                            })
                        })
                        .set_description(
                            "Select the name of the agency you want to receive reminders for again",
                        )
                        .set_custom_id(&format!("{}-remove-filter", user_id))
                        .set_user(user_id)
                        .set_options(
                            LAUNCH_AGENCIES
                                .iter()
                                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                                .filter(|(k, _)| filters_clone.contains(k))
                                .collect(),
                        )
                        .build()
                        .unwrap()
                        .listen(
                            http,
                            &Interaction::MessageComponent(button_click),
                            data,
                        )
                        .await;
                    })
                },
            );
        }

        em.add_option(
            &ButtonType {
                label: "Back to the filters page".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(BACK_EMOJI)),
            },
            move |_| {
                let ses = ses.clone();
                Box::pin(async move { filters_page(ses.clone(), id).await })
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

        let (description, allow_filters) = match id {
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
                            write!(
                                text,
                                "\n`{}`",
                                LAUNCH_AGENCIES
                                    .get(filter.as_str())
                                    .unwrap_or(&"unknown agency")
                            )
                            .expect("write to String: can't fail");
                        }
                        (
                            text,
                            settings
                                .allow_filters
                                .clone(),
                        )
                    },
                    _ => {
                        (
                            "No agency allow filters have been set yet".to_owned(),
                            Vec::new(),
                        )
                    },
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
                            write!(
                                text,
                                "\n`{}`",
                                LAUNCH_AGENCIES
                                    .get(filter.as_str())
                                    .unwrap_or(&"unknown agency")
                            )
                            .expect("write to String: can't fail");
                        }
                        (
                            text,
                            settings
                                .allow_filters
                                .clone(),
                        )
                    },
                    _ => {
                        (
                            "No agency allow filters have been set yet".to_owned(),
                            Vec::new(),
                        )
                    },
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
        let allow_filters_clone = allow_filters.clone();
        em.add_option(
            &ButtonType {
                style: ButtonStyle::Primary,
                label: "Add allow filter".to_owned(),
                emoji: Some(PROGRADE.clone()),
            },
            move |button_click| {
                let add_ses = add_ses.clone();
                let allow_filters_clone = allow_filters_clone.clone();
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
                                add_filter(
                                    &wait_ses.clone(),
                                    id,
                                    choice,
                                    "allow_filters",
                                )
                                .await;
                            } else {
                                eprintln!("select menu returned unknown choice")
                            }
                            allow_filters_page(wait_ses.clone(), id).await
                        })
                    })
                    .set_description(
                        "Select the name of the agency you specifically want to get reminders for",
                    )
                    .set_custom_id(&format!("{}-add-allow-filter", user_id))
                    .set_user(user_id)
                    .make_ephemeral()
                    .set_options(
                        LAUNCH_AGENCIES
                            .iter()
                            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                            .filter(|(k, _)| !allow_filters_clone.contains(k))
                            .collect(),
                    )
                    .build()
                    .unwrap()
                    .listen(
                        http,
                        &Interaction::MessageComponent(button_click),
                        data,
                    )
                    .await;
                })
            },
        );

        if !allow_filters.is_empty() {
            let remove_ses = ses.clone();
            let allow_filters_clone = allow_filters.clone();
            em.add_option(
                &ButtonType {
                    label: "Remove allow filter".to_owned(),
                    style: ButtonStyle::Primary,
                    emoji: Some(RETROGRADE.clone())
                },
                move |button_click| {
                    let remove_ses = remove_ses.clone();
                    let allow_filters_clone = allow_filters_clone.clone();
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
                                        eprintln!("select menu returned unknown choice")
                                    }
                                    allow_filters_page(wait_ses.clone(), id).await
                                })
                        })
                            .set_description("Select the name of the agency you do not want to receive reminders for again")
                            .set_custom_id(&format!("{}-remove-allow-filter", user_id))
                        .set_user(user_id)
                            .make_ephemeral()
                            .set_options(
                                LAUNCH_AGENCIES
                                    .iter()
                                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                                    .filter(|(k, _)| allow_filters_clone.contains(k))
                                    .collect()
                            )
                            .build()
                            .unwrap()
                            .listen(http, &Interaction::MessageComponent(button_click), data).await;
                    })
                },
            );
        }

        em.add_option(
            &ButtonType {
                label: "Back to the filters page".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(BACK_EMOJI)),
            },
            move |_| {
                let ses = ses.clone();
                Box::pin(async move { filters_page(ses.clone(), id).await })
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

fn payload_filters_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let db = if let Some(db_res) = get_db(&ses).await {
            db_res
        } else {
            return;
        };

        let (description, payload_filters) = match id {
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
                            .payload_filters
                            .is_empty() =>
                    {
                        let mut text = "The following payload filters have been added:".to_owned();
                        for filter in &settings.payload_filters {
                            write!(
                                text,
                                "\n`{}`",
                                regex_filter_to_string(filter)
                            )
                            .expect("write to String: can't fail");
                        }
                        (
                            text,
                            settings
                                .payload_filters
                                .clone(),
                        )
                    },
                    _ => {
                        (
                            "No payload filters have been added yet".to_owned(),
                            Vec::new(),
                        )
                    },
                }
            },
            ID::User(user_id) => {
                let settings_res = get_user_settings(&db, user_id.into()).await;
                match settings_res {
                    Ok(settings)
                        if !settings
                            .payload_filters
                            .is_empty() =>
                    {
                        let mut text = "The following payload filters have been added:".to_owned();
                        for filter in &settings.payload_filters {
                            write!(
                                text,
                                "\n`{}`",
                                regex_filter_to_string(filter)
                            )
                            .expect("write to String: can't fail");
                        }
                        (
                            text,
                            settings
                                .payload_filters
                                .clone(),
                        )
                    },
                    _ => {
                        (
                            "No payload filters have been added yet".to_owned(),
                            Vec::new(),
                        )
                    },
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
        em.add_non_update_option(
            &ButtonType {
                style: ButtonStyle::Primary,
                label: "Add payload filter".to_owned(),
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

                    Modal::builder(move |inputs| {
                        let wait_ses = wait_ses.clone();
                        Box::pin(async move {
                            if !inputs.is_empty() {
                                add_filter(
                                    &wait_ses.clone(),
                                    id,
                                    filter_from_string_input(
                                        inputs
                                            .first()
                                            .expect("modal did not return an input value")
                                            .clone()
                                            .1,
                                    ),
                                    "payload_filters",
                                )
                                .await;
                            }
                            payload_filters_page(wait_ses.clone(), id).await
                        })
                    })
                    .set_title("Payload filter modal")
                    .set_custom_id(&format!(
                        "{}-add-payload-filter",
                        user_id
                    ))
                    .set_user(user_id)
                    .add_field(
                        Field::new(
                            "added_payload_filter",
                            "New payload filter",
                        )
                        .set_max_length(20)
                        .set_min_length(3)
                        .set_placeholder("Put in a word or regex to filter out payloads")
                        .set_style(InputTextStyle::Short)
                        .set_required(),
                    )
                    .build()
                    .unwrap()
                    .listen(
                        http,
                        &Interaction::MessageComponent(button_click),
                        data,
                    )
                    .await;
                })
            },
        );

        if !payload_filters.is_empty() {
            let remove_ses = ses.clone();
            let payload_filters_clone = payload_filters.clone();
            em.add_option(
                &ButtonType {
                    label: "Remove payload filter".to_owned(),
                    style: ButtonStyle::Primary,
                    emoji: Some(RETROGRADE.clone()),
                },
                move |button_click| {
                    let remove_ses = remove_ses.clone();
                    let payload_filters_clone = payload_filters_clone.clone();
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
                                remove_filter(
                                    &wait_ses.clone(),
                                    id,
                                    choice,
                                    "payload_filters",
                                )
                                .await;
                                payload_filters_page(wait_ses.clone(), id).await
                            })
                        })
                        .set_description("Select payload filter you want to remove")
                        .set_custom_id(&format!(
                            "{}-remove-payload-filter",
                            user_id
                        ))
                        .set_user(user_id)
                        .make_ephemeral()
                        .set_options(
                            payload_filters_clone
                                .iter()
                                .map(|r| {
                                    (
                                        r.as_str()
                                            .to_owned(),
                                        regex_filter_to_string(r),
                                    )
                                })
                                .collect(),
                        )
                        .build()
                        .unwrap()
                        .listen(
                            http,
                            &Interaction::MessageComponent(button_click),
                            data,
                        )
                        .await;
                    })
                },
            );
        }

        em.add_option(
            &ButtonType {
                label: "Back to the filters page".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(BACK_EMOJI)),
            },
            move |_| {
                let ses = ses.clone();
                Box::pin(async move { filters_page(ses.clone(), id).await })
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

pub fn mentions_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let db = if let Some(db_res) = get_db(&ses).await {
            db_res
        } else {
            return;
        };

        let (description, mentions) = match id {
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
                                write!(text, "\n`{}`", role.name)
                                    .expect("write to String: can't fail");
                            } else {
                                remove_mention(&ses, id, *role_id).await
                            }
                        }
                        (
                            text,
                            settings
                                .mentions
                                .clone(),
                        )
                    },
                    _ => {
                        (
                            "No role mentions have been set yet".to_owned(),
                            Vec::new(),
                        )
                    },
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
        let mentions_clone = mentions.clone();
        em.add_option(
            &ButtonType {
                label: "Add mention".to_owned(),
                style: ButtonStyle::Primary,
                emoji: Some(PROGRADE.clone()),
            },
            move |button_click| {
                let add_ses = add_ses.clone();
                let mentions_clone = mentions_clone.clone();
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
                        Some(mentions_clone),
                        None,
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

        if !mentions.is_empty() {
            let remove_ses = ses.clone();
            let mentions_clone = mentions.clone();
            em.add_option(
                &ButtonType {
                    label: "Remove mention".to_owned(),
                    style: ButtonStyle::Primary,
                    emoji: Some(RETROGRADE.clone()),
                },
                move |button_click| {
                    let remove_ses = remove_ses.clone();
                    let mentions_clone = mentions_clone.clone();
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
                            None,
                            Some(mentions_clone),
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

pub fn other_page(
    ses: Arc<RwLock<EmbedSession>>,
    id: ID,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let db = if let Some(db_res) = get_db(&ses).await {
            db_res
        } else {
            return;
        };

        let mut scrub_notifications = State::Off;
        let mut outcome_notifications = State::Off;
        let mut mentions = State::Off;
        let mut description = String::new();

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
                    toggle_setting(
                        &mentions_ses,
                        id,
                        "mention_others",
                        !mentions.as_ref(),
                    )
                    .await;
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

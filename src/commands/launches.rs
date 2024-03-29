use std::sync::Arc;

use chrono::Utc;
use itertools::Itertools;
use okto_framework::macros::command;
use serenity::{
    all::InteractionResponseFlags,
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
        CreateEmbedFooter,
        CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    framework::standard::CommandResult,
    model::{
        application::{
            ButtonStyle,
            CommandInteraction,
        },
        channel::ReactionType,
        id::EmojiId,
        Timestamp,
    },
    prelude::{
        Context,
        RwLock,
    },
};

use crate::{
    events::statefulembed::{
        ButtonType,
        EmbedSession,
        StatefulEmbed,
    },
    models::{
        caches::LaunchesCacheKey,
        launches::{
            LaunchData,
            LaunchStatus,
        },
    },
    utils::{
        constants::*,
        cutoff_on_last_dot,
        default_embed,
        format_duration,
        launches::*,
        StandardButton,
    },
};

#[command]
/// Get information about the next launch that has been marked as certain
#[options(
    {
        option_type: String,
        name: "lsp",
        description: "Launch Service Provider to filter the launches on",
        required: false
    },
    {
        option_type: String,
        name: "rocket",
        description: "Rocket name to filter the launches on",
        required: false
    }
)]
async fn nextlaunch(ctx: &Context, interaction: &CommandInteraction) -> CommandResult {
    let mut launches: Vec<LaunchData> = {
        if let Some(launch_cache) = ctx
            .data
            .read()
            .await
            .get::<LaunchesCacheKey>()
        {
            Ok(launch_cache
                .read()
                .await
                .to_vec())
        } else {
            Err("Can't get launch cache")
        }
    }?
    .into_iter()
    .filter(|l| l.status == LaunchStatus::Go)
    .collect();

    if launches.is_empty() {
        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(InteractionResponseFlags::EPHEMERAL)
                        .embed(default_embed(
                            "I found no upcoming launches that have been marked as certain :(",
                            false,
                        )),
                ),
            )
            .await?;
        return Ok(());
    }

    launches = match filter_launches(launches, interaction) {
        Ok(ls) => ls,
        Err(err) => {
            interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().flags(InteractionResponseFlags::EPHEMERAL)
                                .embed(default_embed(
                                    &if let FilterErrorType::Invalid = err {
                                        "This is not a valid filter, please take a look at those listed in `/filtersinfo`".to_owned()
                                    } else {
                                        format!("This {err} does not have any upcoming launches listed as certain :(")
                                    },
                                    false
                                ))
                        )
                    ,
                )
                .await?;
            return Ok(());
        },
    };

    let launch = &launches[0];

    let mut window = format_duration(launch.launch_window, true);
    if window.is_empty() {
        window.push_str("instantaneous")
    }

    let mut em = CreateEmbed::new()
        .color(DEFAULT_COLOR)
        .author(CreateEmbedAuthor::new("Next Launch").icon_url(DEFAULT_ICON))
        .timestamp(
            Timestamp::from_unix_timestamp(
                launch
                    .net
                    .timestamp(),
            )
            .expect("Invalid timestamp"),
        )
        .title(format!(
            "{}\nStatus: {}",
            &launch.vehicle,
            launch
                .status
                .as_str()
        ))
        .description(format!(
            "**Payload:** {}\n\
**NET:** <t:{}>\n\
**Provider:** {}\n\
**Location:** {}\n\
**Launch Window:** {}",
            &launch.payload,
            launch
                .net
                .timestamp(),
            &launch.lsp,
            &launch.location,
            window
        ))
        .field(
            "Time until launch:",
            format_duration(
                launch.net - Utc::now().naive_utc(),
                true,
            ),
            false,
        );

    if let Some(img) = &launch.rocket_img {
        em = em.thumbnail(img);
    }

    if let Some(links) = format_links(&launch.vid_urls) {
        em = em.field("links", links, false);
    }

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(em)),
        )
        .await?;

    Ok(())
}

fn list_page(
    session: Arc<RwLock<EmbedSession>>,
    list: Vec<LaunchData>,
    page_num: usize,
    all: bool,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let launches = if all {
            list.clone()
        } else {
            list.iter()
                .filter(|l| l.status == LaunchStatus::Go)
                .cloned()
                .collect()
        };

        let min = page_num * 10;
        let max_page = (launches.len() - 1) / 10;

        let top = if (page_num * 10 + 10) < launches.len() {
            page_num * 10 + 10
        } else {
            launches.len()
        };

        let mut em = StatefulEmbed::new_with_embed(
            session.clone(),
            CreateEmbed::new().color(DEFAULT_COLOR)
                    .author(CreateEmbedAuthor::new("List of upcoming launches").icon_url(DEFAULT_ICON))
                    .timestamp(Utc::now())
                    .footer(CreateEmbedFooter::new(format!("Source: {LAUNCH_LIBRARY_URL}",))
                    ).description(if all {"
            This list shows the upcoming launches (max 100), both certain and uncertain.\n\
            Use the arrow reactions to get to other pages and the green reaction to filter on only the launches that are certain.
            "} else {"
            This list shows upcoming launches that are certain.\n\
            Use the arrow reactions to get to other pages and the red reaction to get all the launches.
            "}).fields(launches[min..top].iter().map(|launch| (format!(
                "{}: {} - {}",
                launch.id,
                &launch.vehicle,
                launch
                    .status
                    .as_str()
            ),
            format!(
                "**Payload:** {}\n**NET:** <t:{}>\n**Provider:** {}\n**Location:** {}",
                &launch.payload,
                launch
                    .net
                    .timestamp(),
                &launch.lsp,
                &launch.location
            ),
            false,)))
        );

        if page_num > 0 {
            let first_page_launches = list.clone();
            let first_page_session = session.clone();
            em.add_option(
                &StandardButton::First.to_button(),
                move |_| {
                    let first_page_session = first_page_session.clone();
                    let first_page_launches = first_page_launches.clone();
                    Box::pin(async move {
                        list_page(
                            first_page_session.clone(),
                            first_page_launches.clone(),
                            0,
                            true,
                        )
                        .await
                    })
                },
            );
        }

        if page_num > 0 {
            let last_page_launches = list.clone();
            let last_page_session = session.clone();
            em.add_option(
                &StandardButton::Back.to_button(),
                move |_| {
                    let last_page_launches = last_page_launches.clone();
                    let last_page_session = last_page_session.clone();
                    Box::pin(async move {
                        list_page(
                            last_page_session.clone(),
                            last_page_launches.clone(),
                            page_num - 1,
                            true,
                        )
                        .await
                    })
                },
            );
        }

        if all
            && launches
                .iter()
                .any(|l| l.status == LaunchStatus::Go)
        {
            let certain_page_launches = list.clone();
            let certain_page_session = session.clone();
            em.add_option(
                &ButtonType {
                    label: "Only certain launches".to_owned(),
                    style: ButtonStyle::Primary,
                    emoji: Some(ReactionType::Custom {
                        animated: false,
                        name: Some("certain".to_owned()),
                        id: EmojiId::from(CERTAIN_EMOJI),
                    }),
                },
                move |_| {
                    let certain_page_session = certain_page_session.clone();
                    let certain_page_launches = certain_page_launches.clone();
                    Box::pin(async move {
                        list_page(
                            certain_page_session.clone(),
                            certain_page_launches.clone(),
                            0,
                            false,
                        )
                        .await
                    })
                },
            );
        } else if !all {
            let uncertain_page_launches = list.clone();
            let uncertain_page_session = session.clone();
            em.add_option(
                &ButtonType {
                    label: "Include uncertain launches".to_owned(),
                    style: ButtonStyle::Primary,
                    emoji: Some(ReactionType::Custom {
                        animated: false,
                        name: Some("uncertain".to_owned()),
                        id: EmojiId::from(UNCERTAIN_EMOJI),
                    }),
                },
                move |_| {
                    let uncertain_page_session = uncertain_page_session.clone();
                    let uncertain_page_launches = uncertain_page_launches.clone();
                    Box::pin(async move {
                        list_page(
                            uncertain_page_session.clone(),
                            uncertain_page_launches.clone(),
                            0,
                            true,
                        )
                        .await
                    })
                },
            );
        }

        if page_num < max_page {
            let next_page_launches = list.clone();
            let next_page_session = session.clone();
            em.add_option(
                &StandardButton::Forward.to_button(),
                move |_| {
                    let next_page_launches = next_page_launches.clone();
                    let next_page_session = next_page_session.clone();
                    Box::pin(async move {
                        list_page(
                            next_page_session.clone(),
                            next_page_launches.clone(),
                            page_num + 1,
                            true,
                        )
                        .await
                    })
                },
            );
        }

        if page_num < max_page {
            let final_page_launches = list;
            let final_page_session = session.clone();
            em.add_option(
                &StandardButton::Last.to_button(),
                move |_| {
                    let final_page_launches = final_page_launches.clone();
                    let final_page_session = final_page_session.clone();
                    Box::pin(async move {
                        list_page(
                            final_page_session.clone(),
                            final_page_launches.clone(),
                            final_page_launches.len() / 10 - 1,
                            true,
                        )
                        .await
                    })
                },
            );
        }

        em.add_option(
            &StandardButton::Exit.to_button(),
            move |_| {
                let session = session.clone();
                Box::pin(async move {
                    let lock = session
                        .read()
                        .await;
                    let _ = lock
                        .interaction
                        .delete_response(&lock.http)
                        .await;
                })
            },
        );

        let res = em
            .show()
            .await;
        if res.is_err() {
            dbg!(res.unwrap_err());
        }
    })
}

#[command]
/// Get a list of the next 100 upcoming launches
#[options(
    {
        option_type: String,
        name: "lsp",
        description: "Launch Service Provider to filter the launches on"
    },
    {
        option_type: String,
        name: "rocket",
        description: "Rocket name to filter the launches on"
    }
)]
async fn listlaunches(ctx: &Context, interaction: &CommandInteraction) -> CommandResult {
    let mut launches: Vec<LaunchData> = {
        if let Some(launch_cache) = ctx
            .data
            .read()
            .await
            .get::<LaunchesCacheKey>()
        {
            Ok(launch_cache
                .read()
                .await
                .to_vec())
        } else {
            Err("Can't get launch cache")
        }
    }?;

    if launches.is_empty() {
        return Err("No launches found".into());
    }

    launches = match filter_launches(launches, interaction) {
        Ok(ls) => ls,
        Err(err) => {
            interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().flags(InteractionResponseFlags::EPHEMERAL)
                            .embed(default_embed(
                                &if let FilterErrorType::Invalid = err {
                                    "This is not a valid filter, please take a look at those listed in `/filtersinfo`".to_owned()
                                } else {
                                    format!("This {err} does not have any upcoming launches :(")
                                },
                                false
                            ))
                        )
                    ,
                )
                .await?;
            return Ok(());
        },
    };

    let session = EmbedSession::new(ctx, interaction.clone(), false).await?;

    list_page(session, launches, 0, true).await;

    Ok(())
}

#[command]
/// Get more detailed information about a launch
#[options({
    option_type: Integer,
    name: "launch",
    description: "The number of the launch to get more information about",
    required: true,
})]
async fn launchinfo(ctx: &Context, interaction: &CommandInteraction) -> CommandResult {
    let launches: Vec<LaunchData> = {
        if let Some(launch_cache) = ctx
            .data
            .read()
            .await
            .get::<LaunchesCacheKey>()
        {
            Ok(launch_cache
                .read()
                .await
                .to_vec())
        } else {
            Err("Can't get launch cache")
        }
    }?;

    if launches.is_empty() {
        return Err("No launches found".into());
    }

    let launch_id = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "launch")
        .and_then(|o| {
            o.value
                .as_i64()
                .map(|i| {
                    let o: i32 = i
                        .try_into()
                        .expect("Got a launch id that was too big to be possible");
                    o
                })
        })
        .ok_or("No launch id provided while it was a required argument")?;

    let Some(launch) = launches
        .into_iter()
        .find(|l| l.id == launch_id)
    else {
        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(InteractionResponseFlags::EPHEMERAL)
                        .embed(default_embed(
                            "No launch was found with that ID :(",
                            false,
                        )),
                ),
            )
            .await?;
        return Ok(());
    };
    let mut window = format_duration(launch.launch_window, true);
    if window.is_empty() {
        window.push_str("instantaneous")
    }

    let mut em = CreateEmbed::new()
        .color(DEFAULT_COLOR)
        .author(CreateEmbedAuthor::new("Detailed info").icon_url(DEFAULT_ICON))
        .timestamp(
            Timestamp::from_unix_timestamp(
                launch
                    .net
                    .timestamp(),
            )
            .expect("Invalid timestamp"),
        )
        .title(format!(
            "{}\nStatus: {}",
            &launch.vehicle,
            launch
                .status
                .as_str()
        ))
        .field(
            "NET:",
            format!(
                "<t:{}>",
                launch
                    .net
                    .timestamp()
            ),
            false,
        )
        .field(
            "General information",
            format!(
                "**Payload:** {}\n\
                            **Provider:** {}\n\
                            **Location:** {}\n\
                            **Launch Window:** {}",
                &launch.payload, &launch.lsp, &launch.location, window
            ),
            false,
        );

    if launch.net > Utc::now().naive_utc() {
        em = em.field(
            "Time until launch:",
            format_duration(
                launch.net - Utc::now().naive_utc(),
                true,
            ),
            false,
        );
    }

    let description = if launch
        .mission_description
        .len()
        > 1024
    {
        format!(
            "{} ...\nlength is too long for discord",
            cutoff_on_last_dot(&launch.mission_description, 980)
        )
    } else {
        launch
            .mission_description
            .clone()
    };

    em = em.field("Desciption:", description, false);

    if let Some(img) = &launch.rocket_img {
        em = em.thumbnail(img);
    }

    if let Some(links) = format_links(&launch.vid_urls) {
        em = em.field("vids", links, false);
    }

    em = em.field(
                    "links",
                    format!(
                        "**My Source:** [The Space Devs]({0})\n\
                        **Rocket Watch:** [rocket.watch](https://rocket.watch/#id={1})\n\
                        **Go4Liftoff:** [go4liftoff.com](https://go4liftoff.com/#page=singleLaunch?filters=launchID={1})",
                        LAUNCH_LIBRARY_URL, launch.id,
                    ),
                    false,
                );

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().embed(em)),
        )
        .await?;

    Ok(())
}

#[command]
/// Get a list of all things you can filter launches on
async fn filtersinfo(ctx: &Context, interaction: &CommandInteraction) -> CommandResult {
    let sorted_agencies = LAUNCH_AGENCIES
        .iter()
        .sorted()
        .map(|(k, v)| (*k, *v))
        .collect::<Vec<(&str, &str)>>();
    let half_agency_count = sorted_agencies.len() / 2;

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(
                    CreateEmbed::new()
                        .color(DEFAULT_COLOR)
                        .author(CreateEmbedAuthor::new("Filters Info").icon_url(DEFAULT_ICON))
                        .timestamp(Utc::now())
                        .title("The following filters can be used to filter launches:")
                        .field(
                            "Vehicles:",
                            LAUNCH_VEHICLES
                                .keys()
                                .sorted()
                                .join(", "),
                            false,
                        )
                        .field(
                            "Launch Service Provider abbreviations with their full names (part 1):",
                            sorted_agencies
                                .iter()
                                .take(half_agency_count)
                                .map(|(k, v)| format!("**{k}**: {v}"))
                                .collect::<Vec<String>>()
                                .join("\n"),
                            false,
                        )
                        .field(
                            "Launch Service Provider abbreviations with their full names (part 2):",
                            sorted_agencies
                                .iter()
                                .skip(half_agency_count)
                                .map(|(k, v)| format!("**{k}**: {v}"))
                                .collect::<Vec<String>>()
                                .join("\n"),
                            false,
                        ),
                ),
            ),
        )
        .await?;

    Ok(())
}

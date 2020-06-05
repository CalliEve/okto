use std::sync::Arc;

use chrono::Utc;
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage},
    framework::standard::{
        macros::{command, group},
        Args,
        CommandResult,
    },
    model::{
        channel::{Message, ReactionType},
        id::EmojiId,
    },
    prelude::{Context, RwLock},
};

use crate::{
    events::statefulembed::{EmbedSession, StatefulEmbed},
    models::{
        caches::LaunchesCacheKey,
        launches::{LaunchData, LaunchStatus},
    },
    utils::{constants::*, default_embed, format_duration, launches::*},
};

const FINAL_PAGE_EMOJI: &'static str = "⏭";
const NEXT_PAGE_EMOJI: &'static str = "▶";
const LAST_PAGE_EMOJI: &'static str = "◀";
const FIRST_PAGE_EMOJI: &'static str = "⏮";
const CERTAIN_EMOJI: u64 = 447805610482728964;
const UNCERTAIN_EMOJI: u64 = 447805624923717642;
const LAUNCH_LIBRARY_URL: &'static str = "http://www.launchlibrary.net/";

#[group]
#[commands(nextlaunch, listlaunches)]
struct Launches;

#[command]
#[aliases(nl)]
fn nextlaunch(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut launches: Vec<LaunchData> = {
        if let Some(launch_cache) = ctx.data.read().get::<LaunchesCacheKey>() {
            Ok(launch_cache.read().to_vec())
        } else {
            Err("Can't get launch cache")
        }
    }?
    .into_iter()
    .filter(|l| l.status == LaunchStatus::Go)
    .collect();

    if launches.is_empty() {
        msg.channel_id
            .send_message(&ctx.http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    default_embed(
                        e,
                        "I found no upcoming launches that have been marked as certain :(",
                        false,
                    )
                })
            })?;
        return Ok(());
    }

    let launch = &launches[0];

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.color(DEFAULT_COLOR)
                    .author(|a: &mut CreateEmbedAuthor| {
                        a.name("Next Launch").icon_url(DEFAULT_ICON)
                    })
                    .timestamp(&Utc::now())
                    .title(format!(
                        "{}\n\nStatus: {}",
                        &launch.vehicle,
                        launch.status.to_emoji()
                    ))
                    .description(format!(
                        "**Payload:** {}
                        **NET:** {}
                        **Provider:** {}
                        **Location:** {}
                        **Launch Window:** {}",
                        &launch.payload,
                        launch.net.format("%Y-%m-%d %H:%M:%S"),
                        &launch.lsp,
                        &launch.location,
                        format_duration(launch.launch_window)
                    ))
                    .field(
                        "Time until launch:",
                        format_duration(launch.net - Utc::now().naive_utc()),
                        false,
                    );

                if let Some(img) = &launch.rocket_img {
                    e.thumbnail(img);
                }

                if let Some(links) = format_links(&launch.vid_urls) {
                    e.field("links", links, false);
                }

                e
            })
        })?;

    Ok(())
}

fn list_page(
    session: Arc<RwLock<EmbedSession>>,
    list: Vec<LaunchData>,
    page_num: usize,
    all: bool,
) {
    let launches = if all {
        list.clone()
    } else {
        list.iter()
            .filter(|l| l.status == LaunchStatus::Go)
            .map(|l| l.clone())
            .collect()
    };

    let min = page_num * 10;
    let max_page = (launches.len() - 1) / 10;

    let top = if (page_num * 10 + 10) < launches.len() {
        page_num * 10 + 10
    } else {
        launches.len()
    };

    let mut em = StatefulEmbed::new_with(session.clone(), |e: &mut CreateEmbed| {
        e.color(DEFAULT_COLOR)
            .author(|a: &mut CreateEmbedAuthor| {
                a.icon_url(DEFAULT_ICON).name("List of upcoming launches")
            })
            .timestamp(&Utc::now())
            .footer(|f: &mut CreateEmbedFooter| f.text(format!("Source: {}", LAUNCH_LIBRARY_URL)));

        if all {
            e.description("
            This list shows the next 100 upcoming launches, both certain and uncertain.
            Use the arrow reactions to get to other pages and the green reaction to filter on only the launches that are certain.
            ");
        } else {
            e.description("
            This list shows upcoming launches that are certain.
            Use the arrow reactions to get to other pages and the red reaction to get all the launches.
            ");
        }

        for i in min..top {
            e.field(
                format!(
                    "{}: {} {}",
                    i + 1,
                    &launches[i].vehicle,
                    launches[i].status.to_emoji()
                ),
                format!(
                    "**Payload:** {}\n**Date:** {}\n**Time:** {}\n**Provider:** {}\n**Location:** {}",
                    &launches[i].payload,
                    launches[i].net.format("%d %B %Y"),
                    launches[i].net.format("%T"),
                    &launches[i].lsp,
                    &launches[i].location
                ),
                false,
            );
        }
        e
    });

    if page_num > 0 {
        let first_page_launches = list.clone();
        let first_page_session = session.clone();
        em.add_option(&ReactionType::from(FIRST_PAGE_EMOJI), move || {
            list_page(
                first_page_session.clone(),
                first_page_launches.clone(),
                0,
                true,
            )
        });
    }

    if page_num > 0 {
        let last_page_launches = list.clone();
        let last_page_session = session.clone();
        em.add_option(&ReactionType::from(LAST_PAGE_EMOJI), move || {
            list_page(
                last_page_session.clone(),
                last_page_launches.clone(),
                page_num - 1,
                true,
            )
        });
    }

    if all {
        let certain_page_launches = list.clone();
        let certain_page_session = session.clone();
        em.add_option(
            &ReactionType::Custom {
                animated: false,
                name: Some("certain".to_owned()),
                id: EmojiId::from(CERTAIN_EMOJI),
            },
            move || {
                list_page(
                    certain_page_session.clone(),
                    certain_page_launches.clone(),
                    0,
                    false,
                )
            },
        );
    } else {
        let uncertain_page_launches = list.clone();
        let uncertain_page_session = session.clone();
        em.add_option(
            &ReactionType::Custom {
                animated: false,
                name: Some("uncertain".to_owned()),
                id: EmojiId::from(UNCERTAIN_EMOJI),
            },
            move || {
                list_page(
                    uncertain_page_session.clone(),
                    uncertain_page_launches.clone(),
                    0,
                    true,
                )
            },
        );
    }

    if page_num < max_page {
        let next_page_launches = list.clone();
        let next_page_session = session.clone();
        em.add_option(&ReactionType::from(NEXT_PAGE_EMOJI), move || {
            list_page(
                next_page_session.clone(),
                next_page_launches.clone(),
                page_num + 1,
                true,
            )
        });
    }

    if page_num < max_page {
        let final_page_launches = list.clone();
        let final_page_session = session.clone();
        em.add_option(&ReactionType::from(FINAL_PAGE_EMOJI), move || {
            list_page(
                final_page_session.clone(),
                final_page_launches.clone(),
                final_page_launches.len() / 10 - 1,
                true,
            )
        });
    }

    let res = em.show();
    if res.is_err() {
        dbg!(res.unwrap_err());
    }
}

#[command]
#[aliases(ll)]
fn listlaunches(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut launches: Vec<LaunchData> = {
        if let Some(launch_cache) = ctx.data.read().get::<LaunchesCacheKey>() {
            Ok(launch_cache.read().to_vec())
        } else {
            Err("Can't get launch cache")
        }
    }?;

    if launches.len() == 0 {
        return Err("No launches found".into());
    }

    let session = EmbedSession::new(&ctx.http, msg.channel_id, msg.author.id).show(&ctx)?;

    list_page(session, launches, 0, true);

    Ok(())
}

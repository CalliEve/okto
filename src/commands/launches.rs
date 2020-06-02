use chrono::{NaiveDateTime, Utc};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage},
    framework::standard::{
        macros::{command, group},
        Args,
        CommandResult,
    },
    model::channel::Message,
    prelude::Context,
};

use crate::{
    models::{
        caches::LaunchesCacheKey,
        launches::{LaunchData, LaunchStatus},
    },
    utils::{constants::*, cutoff_on_last_dot, default_embed, format_duration, launches::*},
};

#[group]
#[commands(nextlaunch)]
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

    launches.sort_by_key(|l| l.net);

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
                        true,
                    );

                if let Some(img) = &launch.rocket_img {
                    e.thumbnail(img);
                }

                if let Some(links) = format_links(&launch.vid_urls) {
                    e.field("links", links, true);
                }

                e
            })
        })?;

    Ok(())
}

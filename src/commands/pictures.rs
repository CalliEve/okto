use std::collections::HashMap;

use chrono::{
    Duration,
    TimeZone,
    Utc,
};
use okto_framework::macros::command;
use rand::Rng;
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
        CreateEmbedFooter,
        EditInteractionResponse,
    },
    framework::standard::CommandResult,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction,
        InteractionResponseType,
    },
    prelude::Context,
};

use crate::{
    models::pictures::*,
    utils::{
        constants::*,
        default_embed,
        error_log,
        other::cutoff_on_last_dot,
        pictures::*,
    },
};

#[command]
/// Get a picture of Earth from the NOAA DSCOVR spacecraft
#[options(
    {
        option_type: String,
        name: "image-version",
        description: "natural or enhanced version of the image of our planet earth",
        choices: [
            {
                name: "natural",
                value: "natural"
            },
            {
                name: "enhanced",
                value: "enhanced"
            }
        ]
    }
)]
async fn earthpic(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(&ctx.http, |c| {
            c.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await?;
    let image_type = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "image-version")
        .and_then(|o| {
            o.value
                .clone()
        })
        .and_then(|v| {
            v.as_str()
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "natural".to_owned());

    let opposite = if image_type == "natural" {
        "enhanced"
    } else {
        "natural"
    };

    let epic_image_data: EPICImage = DEFAULT_CLIENT
        .get(
            format!(
                "https://epic.gsfc.nasa.gov/api/{}",
                image_type
            )
            .as_str(),
        )
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<EPICImage>>()
        .await?
        .first()
        .cloned()
        .ok_or("No image received from the EPIC image api")?;

    interaction
        .edit_original_interaction_response(
            &ctx.http,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Earth Picture")
                            .icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .description(format!(
                    "Most recent {} image from the EPIC camera onboard the NOAA DSCOVR spacecraft",
                    image_type
                ))
                    .footer(|f: &mut CreateEmbedFooter| {
                        f.text(format!(
                            "Taken on: {}\nRun this command again with the {} argument!",
                            epic_image_data.date, opposite
                        ))
                    })
                    .image(format!(
                        "https://epic.gsfc.nasa.gov/archive/{}/{}/png/{}.png",
                        image_type,
                        get_date_epic_image(&epic_image_data.date),
                        epic_image_data.image
                    ))
                    .timestamp(Utc::now())
                })
            },
        )
        .await?;

    Ok(())
}

#[command]
/// Get an Astronomy Picture Of the Day
#[options(
    {
        option_type: Boolean,
        name: "today",
        description: "Get todays Astronomy Picture of the Day",
        required: false,
    }
)]
async fn spacepic(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(&ctx.http, |c| {
            c.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await?;

    let now = Utc::today() - Duration::hours(6);

    let date = if interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "today")
        .and_then(|o| {
            o.value
                .clone()
        })
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        now
    } else {
        let start = Utc.ymd(2000, 1, 1);
        let days = (now - start).num_days();
        let day = RNG
            .lock()
            .await
            .gen_range(0..days);
        start + Duration::days(day)
    };

    let mut params = HashMap::new();
    params.insert("hd", "True".to_owned());
    params.insert(
        "date",
        date.format("%Y-%m-%d")
            .to_string(),
    );
    params.insert("api_key", NASA_KEY.to_string());

    let apod_image_req = DEFAULT_CLIENT
        .get("https://api.nasa.gov/planetary/apod")
        .query(&params)
        .send()
        .await?
        .error_for_status();

    if let Err(err) = apod_image_req {
        interaction
            .edit_original_interaction_response(
                &ctx.http,
                |e: &mut EditInteractionResponse| {
                    e.embed(|e: &mut CreateEmbed| {
                        default_embed(e, "The APOD API returned an error so I couldn't get an image :c\nUnfortunately this happens quite often as that api is pretty unstable.", false)
                    })
                }
            ).await?;

        error_log(
            &ctx.http,
            format!("APOD API returned an error: {:?}", err),
        )
        .await;

        return Ok(());
    }

    let apod_image: APODImage = apod_image_req
        .unwrap()
        .json()
        .await?;

    let explanation = apod_image
        .explanation
        .clone()
        .map(|e| {
            e.split("Follow APOD on:")
                .next()
                .unwrap_or("no explanation provided :(")
                .split("digg_url")
                .next()
                .unwrap_or("no explanation provided :(")
                .trim()
                .to_owned()
        })
        .map_or_else(
            || "no explanation provided :(".to_owned(),
            |e| cutoff_on_last_dot(&e, 2040).to_owned(),
        );

    interaction
        .edit_original_interaction_response(
            &ctx.http,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Astronomy Picture of Today")
                            .icon_url(DEFAULT_ICON)
                    })
                    .title(&apod_image.title)
                    .color(DEFAULT_COLOR)
                    .description(explanation)
                    .footer(|f: &mut CreateEmbedFooter| {
                        f.text(format!(
                            "APOD of {}",
                            date.format("%Y-%m-%d")
                        ))
                    })
                    .image(apod_image.url)
                    .timestamp(Utc::now())
                })
            },
        )
        .await?;

    Ok(())
}

#[command]
/// Picks a random sol number and then grabs a random picture made by the Spirit
/// rover on that sol
async fn spirit(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(&ctx.http, |c| {
            c.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await?;

    let (pic, sol) = fetch_rover_camera_picture("spirit", 1..2186).await;

    interaction
        .edit_original_interaction_response(
            &ctx.http,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Random Picture made by the Spirit mars rover")
                            .icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .field(
                        "info:",
                        format!(
                            "**Taken on Sol:** {}\n**Earth Date:** {}\n**Taken by Camera:** {}",
                            sol,
                            pic.earth_date,
                            pic.camera
                                .full_name
                        ),
                        false,
                    )
                    .footer(|f: &mut CreateEmbedFooter| f.text(format!("picture ID: {}", pic.id)))
                    .image(pic.img_src)
                    .timestamp(Utc::now())
                })
            },
        )
        .await?;

    Ok(())
}

#[command]
/// Picks a random sol number and then grabs a random picture made by the
/// Opportunity rover on that sol
async fn opportunity(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(&ctx.http, |c| {
            c.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await?;

    let (pic, sol) = fetch_rover_camera_picture("opportunity", 1..5112).await;

    interaction
        .edit_original_interaction_response(
            &ctx.http,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Random Picture made by the Opportunity mars rover")
                            .icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .field(
                        "info:",
                        format!(
                            "**Taken on Sol:** {}\n**Earth Date:** {}\n**Taken by Camera:** {}",
                            sol,
                            pic.earth_date,
                            pic.camera
                                .full_name
                        ),
                        false,
                    )
                    .footer(|f: &mut CreateEmbedFooter| f.text(format!("picture ID: {}", pic.id)))
                    .image(pic.img_src)
                    .timestamp(Utc::now())
                })
            },
        )
        .await?;

    Ok(())
}

#[command]
/// Picks a random sol number and grabs a random picture made by the Curiosity
/// rover on that sol
async fn curiosity(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(&ctx.http, |c| {
            c.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await?;

    let max_sol = get_max_sol("curiosity").await?;

    let (pic, sol) = fetch_rover_camera_picture("curiosity", 1..max_sol).await;

    interaction
        .edit_original_interaction_response(
            &ctx.http,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Random Picture made by the Curiosity mars rover")
                            .icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .field(
                        "info:",
                        format!(
                            "**Taken on Sol:** {}\n**Earth Date:** {}\n**Taken by Camera:** {}",
                            sol,
                            pic.earth_date,
                            pic.camera
                                .full_name
                        ),
                        false,
                    )
                    .footer(|f: &mut CreateEmbedFooter| f.text(format!("picture ID: {}", pic.id)))
                    .image(pic.img_src)
                    .timestamp(Utc::now())
                })
            },
        )
        .await?;

    Ok(())
}

#[command]
/// Picks a random sol number and grabs a random picture made by the
/// Perseverance rover on that sol.
async fn perseverance(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(&ctx.http, |c| {
            c.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await?;

    let max_sol = get_max_sol("perseverance").await?;

    let (pic, sol) = fetch_rover_camera_picture("perseverance", 1..max_sol).await;

    interaction
        .edit_original_interaction_response(
            &ctx.http,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Random Picture made by the Perseverance mars rover")
                            .icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .field(
                        "info:",
                        format!(
                            "**Taken on Sol:** {}\n**Earth Date:** {}\n**Taken by Camera:** {}",
                            sol,
                            pic.earth_date,
                            pic.camera
                                .full_name
                        ),
                        false,
                    )
                    .footer(|f: &mut CreateEmbedFooter| f.text(format!("picture ID: {}", pic.id)))
                    .image(pic.img_src)
                    .timestamp(Utc::now())
                })
            },
        )
        .await?;

    Ok(())
}

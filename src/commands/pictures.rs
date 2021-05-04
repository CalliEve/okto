use std::collections::HashMap;

use chrono::{
    Duration,
    TimeZone,
    Utc,
};
use rand::{
    seq::SliceRandom,
    Rng,
};
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
        CreateEmbedFooter,
        CreateMessage,
    },
    framework::standard::{
        macros::{
            command,
            group,
        },
        Args,
        CommandResult,
    },
    model::channel::Message,
    prelude::Context,
};

use crate::{
    models::{
        caches::PictureCacheKey,
        pictures::*,
    },
    utils::{
        constants::*,
        other::cutoff_on_last_dot,
        pictures::*,
    },
};

#[group]
#[commands(
    earthpic,
    spacepic,
    hubble,
    spirit,
    opportunity,
    curiosity,
    perseverance
)]
struct Pictures;

#[command]
#[description("Get a picture of Earth from the NOAA DSCOVR spacecraft")]
#[usage(
    "Tell the command to provide the natural or enhanced version of the image, defaults to natural"
)]
async fn earthpic(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;
    let image_type = args
        .quoted()
        .current()
        .map_or("natural", |t| {
            if ["natural", "enhanced"].contains(&t.to_lowercase().as_str()) {
                t
            } else {
                "natural"
            }
        })
        .to_lowercase();

    let opposite = if image_type == "natural" {
        "enhanced"
    } else {
        "natural"
    };

    let epic_image_data: EPICImage = DEFAULT_CLIENT
        .get(format!("https://epic.gsfc.nasa.gov/api/{}", image_type).as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<EPICImage>>()
        .await?
        .first()
        .cloned()
        .ok_or("No image received from the EPIC image api")?;

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.author(|a: &mut CreateEmbedAuthor| a.name("Earth Picture").icon_url(DEFAULT_ICON))
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
                    .timestamp(&Utc::now())
            })
        }).await?;

    Ok(())
}

#[command]
#[description("Get an Astronomy Picture Of the Day")]
#[usage("Picks a random picture, but can be told to get the one from today by giving the \"today\" argument")]
async fn spacepic(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let now = Utc::today();

    let date = if let Some("today") = args.current() {
        now
    } else {
        let start = Utc.ymd(2000, 1, 1);
        let days = (now - start).num_days();
        let day = RNG.lock().await.gen_range(0..days);
        start + Duration::days(day)
    };

    let mut params = HashMap::new();
    params.insert("hd", "True".to_owned());
    params.insert("date", date.format("%Y-%m-%d").to_string());
    params.insert("api_key", NASA_KEY.to_string());

    let apod_image: APODImage = DEFAULT_CLIENT
        .get("https://api.nasa.gov/planetary/apod")
        .query(&params)
        .send()
        .await?
        .error_for_status()?
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

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.author(|a: &mut CreateEmbedAuthor| {
                    a.name("Astronomy Picture of Today").icon_url(DEFAULT_ICON)
                })
                .title(&apod_image.title)
                .color(DEFAULT_COLOR)
                .description(explanation)
                .footer(|f: &mut CreateEmbedFooter| {
                    f.text(format!("APOD of {}", date.format("%Y-%m-%d")))
                })
                .image(apod_image.url)
                .timestamp(&Utc::now())
            })
        })
        .await?;

    Ok(())
}

#[command]
#[description("Picks a random picture from the hubblesite api")]
async fn hubble(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let picn: i32 = if let Some(pic_cache) = ctx.data.read().await.get::<PictureCacheKey>() {
        *pic_cache
            .hubble_pics
            .choose(&mut *RNG.lock().await)
            .ok_or("Could not retrieve a hubble picture from the picture cache")?
    } else {
        return Err("Could not retrieve the picture cache".into());
    };

    let hubble_image_data: HubbleImageSource = DEFAULT_CLIENT
        .get(format!("http://hubblesite.org/api/v3/image/{}", picn).as_str())
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let pic = biggest_image_url(&hubble_image_data);

    let description = &hubble_image_data
        .description
        .as_ref()
        .map_or("no image description provided", |d: &String| {
            cutoff_on_last_dot(d, 2040)
        });

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.author(|a: &mut CreateEmbedAuthor| {
                    a.name("Random Hubble Picture").icon_url(DEFAULT_ICON)
                })
                .color(DEFAULT_COLOR)
                .description(description)
                .footer(|f: &mut CreateEmbedFooter| {
                    f.text(format!("source: hubblesite.org, pic ID: {}", picn))
                })
                .image(pic)
                .timestamp(&Utc::now())
            })
        })
        .await?;

    Ok(())
}

#[command]
#[description("Picks a random sol number and then grabs a random picture made by the Spirit rover on that sol.")]
async fn spirit(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let (pic, sol) = fetch_rover_camera_picture("spirit", 1..2186).await;

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.author(|a: &mut CreateEmbedAuthor| {
                    a.name("Random Picture made by the Spirit mars rover")
                        .icon_url(DEFAULT_ICON)
                })
                .color(DEFAULT_COLOR)
                .field(
                    "info:",
                    format!(
                        "**Taken on Sol:** {}\n**Earth Date:** {}\n**Taken by Camera:** {}",
                        sol, pic.earth_date, pic.camera.full_name
                    ),
                    false,
                )
                .footer(|f: &mut CreateEmbedFooter| f.text(format!("picture ID: {}", pic.id)))
                .image(pic.img_src)
                .timestamp(&Utc::now())
            })
        })
        .await?;

    Ok(())
}

#[command]
#[description("Picks a random sol number and then grabs a random picture made by the Opportunity rover on that sol.")]
async fn opportunity(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let (pic, sol) = fetch_rover_camera_picture("opportunity", 1..5112).await;

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.author(|a: &mut CreateEmbedAuthor| {
                    a.name("Random Picture made by the Opportunity mars rover")
                        .icon_url(DEFAULT_ICON)
                })
                .color(DEFAULT_COLOR)
                .field(
                    "info:",
                    format!(
                        "**Taken on Sol:** {}\n**Earth Date:** {}\n**Taken by Camera:** {}",
                        sol, pic.earth_date, pic.camera.full_name
                    ),
                    false,
                )
                .footer(|f: &mut CreateEmbedFooter| f.text(format!("picture ID: {}", pic.id)))
                .image(pic.img_src)
                .timestamp(&Utc::now())
            })
        })
        .await?;

    Ok(())
}

#[command]
#[description("Picks a random sol number and then grabs a random picture made by the Curiosity rover on that sol.")]
async fn curiosity(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let max_sol = get_max_sol("curiosity").await?;

    let (pic, sol) = fetch_rover_camera_picture("curiosity", 1..max_sol).await;

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.author(|a: &mut CreateEmbedAuthor| {
                    a.name("Random Picture made by the Curiosity mars rover")
                        .icon_url(DEFAULT_ICON)
                })
                .color(DEFAULT_COLOR)
                .field(
                    "info:",
                    format!(
                        "**Taken on Sol:** {}\n**Earth Date:** {}\n**Taken by Camera:** {}",
                        sol, pic.earth_date, pic.camera.full_name
                    ),
                    false,
                )
                .footer(|f: &mut CreateEmbedFooter| f.text(format!("picture ID: {}", pic.id)))
                .image(pic.img_src)
                .timestamp(&Utc::now())
            })
        })
        .await?;

    Ok(())
}

#[command]
#[description("Picks a random sol number and then grabs a random picture made by the Perseverance rover on that sol.")]
async fn perseverance(ctx: &Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let max_sol = get_max_sol("perseverance").await?;

    let (pic, sol) = fetch_rover_camera_picture("perseverance", 1..max_sol).await;

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.author(|a: &mut CreateEmbedAuthor| {
                    a.name("Random Picture made by the Perseverance mars rover")
                        .icon_url(DEFAULT_ICON)
                })
                .color(DEFAULT_COLOR)
                .field(
                    "info:",
                    format!(
                        "**Taken on Sol:** {}\n**Earth Date:** {}\n**Taken by Camera:** {}",
                        sol, pic.earth_date, pic.camera.full_name
                    ),
                    false,
                )
                .footer(|f: &mut CreateEmbedFooter| f.text(format!("picture ID: {}", pic.id)))
                .image(pic.img_src)
                .timestamp(&Utc::now())
            })
        })
        .await?;

    Ok(())
}

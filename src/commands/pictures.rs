use std::collections::HashMap;

use chrono::{Duration, TimeZone, Utc};
use rand::{seq::SliceRandom, thread_rng, Rng};
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
    models::{caches::PictureCacheKey, pictures::*},
    utils::{constants::*, other::cutoff_on_last_dot, pictures::*},
};

#[group]
#[commands(earthpic, spacepic, hubble, spirit, opportunity)]
struct Pictures;

#[command]
fn earthpic(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http);
    let image_type = args
        .quoted()
        .current()
        .map(|t| {
            if !["natural", "enhanced"].contains(&t) {
                "natural"
            } else {
                t
            }
        })
        .unwrap_or("natural")
        .to_lowercase();

    let opposite = if image_type == "natural" {
        "enhanced"
    } else {
        "natural"
    };

    let epic_image_data: EPICImage = DEFAULT_CLIENT
        .get(format!("https://epic.gsfc.nasa.gov/api/{}", image_type).as_str())
        .send()?
        .error_for_status()?
        .json::<Vec<EPICImage>>()?
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
        })?;

    Ok(())
}

#[command]
fn spacepic(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http);

    let now = Utc::today();

    let date = if let Some("today") = args.current() {
        now
    } else {
        let start = Utc.ymd(2000, 1, 1);
        let days = (now - start).num_days();
        let day = thread_rng().gen_range(0, days);
        start + Duration::days(day)
    };

    let mut params = HashMap::new();
    params.insert("hd", "True".to_owned());
    params.insert("date", date.format("%Y-%m-%d").to_string());
    params.insert("api_key", NASA_KEY.to_string());

    let apod_image: APODImage = DEFAULT_CLIENT
        .get("https://api.nasa.gov/planetary/apod")
        .query(&params)
        .send()?
        .error_for_status()?
        .json()?;

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
        .map(|e| cutoff_on_last_dot(&e, 2040).to_owned())
        .unwrap_or("no explanation provided :(".to_owned());

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
        })?;

    Ok(())
}

#[command]
fn hubble(ctx: &mut Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http);

    let picn: i32 = if let Some(pic_cache) = ctx.data.read().get::<PictureCacheKey>() {
        *pic_cache
            .hubble_pics
            .choose(&mut thread_rng())
            .ok_or("Could not retrieve a hubble picture from the picture cache")?
    } else {
        return Err("Could not retrieve the picture cache".into());
    };

    let hubble_image_data: HubbleImageSource = DEFAULT_CLIENT
        .get(format!("http://hubblesite.org/api/v3/image/{}", picn).as_str())
        .send()?
        .error_for_status()?
        .json()?;

    let pic = biggest_image_url(&hubble_image_data);

    let description = if let Some(d) = &hubble_image_data.description {
        cutoff_on_last_dot(d, 2040)
    } else {
        "no image description provided"
    };

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
        })?;

    Ok(())
}

#[command]
fn spirit(ctx: &mut Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http);

    let mut rng = thread_rng();
    let sol: u16 = rng.gen_range(1, 2191);

    let pictures: Vec<MarsRoverPicture> = DEFAULT_CLIENT
        .get(
            format!(
                "https://api.nasa.gov/mars-photos/api/v1/rovers/spirit/photos?sol={}&api_key={}",
                sol,
                NASA_KEY.as_str()
            )
            .as_str(),
        )
        .send()?
        .error_for_status()?
        .json::<MarsRoverPictureRes>()?
        .photos;

    let pic = get_rover_camera_picture(pictures, &mut rng)
        .ok_or(format!("No spirit picture found at sol {}", sol))?;

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
        })?;

    Ok(())
}

#[command]
fn opportunity(ctx: &mut Context, msg: &Message) -> CommandResult {
    let _ = msg.channel_id.broadcast_typing(&ctx.http);

    let mut rng = thread_rng();
    let sol: u16 = rng.gen_range(1, 5112);

    let pictures: Vec<MarsRoverPicture> = DEFAULT_CLIENT
        .get(
            format!(
                "https://api.nasa.gov/mars-photos/api/v1/rovers/opportunity/photos?sol={}&api_key={}",
                sol,
                NASA_KEY.as_str()
            )
            .as_str(),
        )
        .send()?
        .error_for_status()?
        .json::<MarsRoverPictureRes>()?.photos;

    let pic = get_rover_camera_picture(pictures, &mut rng)
        .ok_or(format!("No opportunity picture found at sol {}", sol))?;

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
        })?;

    Ok(())
}

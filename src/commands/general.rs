use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage, EditMessage},
    framework::standard::{
        macros::{command, group},
        CommandResult,
    },
    model::{channel::Message, ModelError},
    prelude::Context,
    Error, Result,
};

use crate::utils::constants::*;

#[group]
#[commands(ping, invite, info, websites, peopleinspace, iss)]
struct General;

#[command]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    let start = Utc::now();
    let mut message = msg.channel_id.send_message(&ctx, |m: &mut CreateMessage| {
        m.embed(|e: &mut CreateEmbed| e.description("🏓 pong..."))
    })?;
    let end = Utc::now();

    let round_trip = end - start;
    let ws_delay = DateTime::<FixedOffset>::from(start) - msg.id.created_at();

    message.edit(ctx, |e: &mut EditMessage| {
        e.embed(|e: &mut CreateEmbed| {
            e.title("Pong!").description(format!(
                "🏓\nws delay: {}ms\napi ping: {}ms",
                ws_delay.num_milliseconds(),
                round_trip.num_milliseconds()
            ))
        })
    })?;

    Ok(())
}

#[command]
fn invite(ctx: &mut Context, msg: &Message) -> CommandResult {
    let msg_res: Result<Message> = msg.channel_id.send_message(&ctx.http, |m: &mut CreateMessage| {
        m.content(format!(
            "**OKTO** | `3.0`\n{}, I hope you enjoy using me on your server!",
            msg.author.name
        ));
        m.embed(|e: &mut CreateEmbed| {
            e.title("Helpful Links")
            .description(
                "**__[Bot Invite](https://discordapp.com/oauth2/authorize?client_id=429306620439166977&scope=bot&permissions=289856)__**
                **__[OKTO server](https://discord.gg/dXPHfPJ)__**"
            )
        })
    });

    if let Err(Error::Model(ModelError::InvalidPermissions(perms))) = msg_res {
        if perms.embed_links() {
            let _ = msg.channel_id.send_message(&ctx.http, |m| {
                m.content("❌ You must give this bot embed permissions ❌")
            });
        }
    }

    Ok(())
}

#[command]
fn info(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id.send_message(&ctx.http, |m: &mut CreateMessage| {
        m.embed(|e: &mut CreateEmbed| {
            e.title("Helpful Links")
            .description(
                "This is a bot to show upcoming launches and provide additional information on everything to do with spaceflight
                **Author:** Callidus#3141
                \n<:discord:314003252830011395>
                [**Support Server**](https://discord.gg/dXPHfPJ)
                [**Rocket Watch server**](https://discord.gg/Hyd4umg)
                \n<:botTag:230105988211015680>
                If you want OKTO on your server, click [**here**](https://discordapp.com/oauth2/authorize?client_id=429306620439166977&scope=bot&permissions=289856)
                If you like OKTO, please [**vote**](https://discordbots.org/bot/429306620439166977/vote) ^-^"
            )
            .author(|a: &mut CreateEmbedAuthor| {
                a.name("Bot Information")
                .icon_url(DEFAULT_ICON)
            })
            .thumbnail(TRANSPARENT_ICON)
            .color(DEFAULT_COLOR)
        })
    })?;
    Ok(())
}

#[command]
fn websites(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.field(
                    "General launch information:",
                    "**Spaceflight Insider:** http://www.spaceflightinsider.com/
                    **Rocket Watch:** https://rocket.watch/
                    **Go4LiftOff:** https://go4liftoff.com/",
                    false,
                )
                .field(
                    "Launch providers:",
                    "**SpaceX:** http://www.spacex.com/
                    **United Launch Alliance:** https://www.ulalaunch.com/
                    **Arianespace:** http://www.arianespace.com/
                    **Rocket Lab:** https://www.rocketlabusa.com/
                    **Roscosmos:** http://en.roscosmos.ru/
                    **Orbital ATK:** https://www.orbitalatk.com/
                    **ISRO:** https://www.isro.gov.in/
                    **NASA:** https://www.nasa.gov/",
                    false,
                )
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("Some websites with more infomation")
                        .icon_url(DEFAULT_ICON)
                })
                .color(DEFAULT_COLOR)
            })
        })?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PersonInSpace {
    pub name: String,
    pub craft: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PeopleInSpaceResp {
    pub people: Vec<PersonInSpace>,
    pub number: i32,
}

#[command]
fn peopleinspace(ctx: &mut Context, msg: &Message) -> CommandResult {
    let pis: PeopleInSpaceResp = DEFAULT_CLIENT
        .get("http://api.open-notify.org/astros.json")
        .send()?
        .error_for_status()?
        .json()?;

    let mut text_vec: Vec<String> = Vec::with_capacity(pis.people.len());
    for person in &pis.people {
        text_vec.push(format!("{}: {}\n", person.name, person.craft))
    }

    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                e.title(format!(
                    "There are currently {} people in space",
                    pis.number
                ))
                .description(text_vec.iter().map(|x| x.as_str()).collect::<String>())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name("People in space").icon_url(DEFAULT_ICON)
                })
                .timestamp(&Utc::now())
                .color(DEFAULT_COLOR)
            })
        })?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ISSLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub velocity: f64,
}

#[command]
fn iss(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut res_msg: Message =
        msg.channel_id
            .send_message(&ctx.http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Position ISS").icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .description("<a:typing:393848431413559296> loading position...")
                })
            })?;

    let iss_pos: ISSLocation = DEFAULT_CLIENT
        .get("https://api.wheretheiss.at/v1/satellites/25544")
        .send()?
        .error_for_status()?
        .json()?;

    let detail_url = format!(
        "https://maps.googleapis.com/maps/api/staticmap?\
        center={0},{1}&\
        maptype=hybrid&\
        size=400x350&\
        markers=color:blue|{0},{1}&\
        zoom=5&\
        key={2}",
        iss_pos.latitude,
        iss_pos.longitude,
        GOOGLE_KEY.as_str()
    );

    let global_url = format!(
        "https://maps.googleapis.com/maps/api/staticmap?\
        center={0},{1}&\
        maptype=hybrid&\
        size=400x400&\
        markers=color:blue|{0},{1}&\
        zoom=1&\
        key={2}",
        iss_pos.latitude,
        iss_pos.longitude,
        GOOGLE_KEY.as_str()
    );

    res_msg.edit(&ctx.http, |m: &mut EditMessage| {
        m.embed(|e: &mut CreateEmbed| {
            e.description(format!(
                "**Latitude:** {0:.5}\n**Longitude:** {1:.5}\n**Altitude:** {2:.3}km\n**Velocity:** {3:.3}km/h",
                iss_pos.latitude, iss_pos.longitude, iss_pos.altitude, iss_pos.velocity
            ))
            .author(|a: &mut CreateEmbedAuthor| a.name("Position ISS").icon_url(DEFAULT_ICON))
            .image(detail_url)
            .thumbnail(global_url)
            .footer(|f: &mut CreateEmbedFooter| f.text("source: wheretheiss.at"))
            .timestamp(&Utc::now())
            .color(DEFAULT_COLOR)
        })
    })?;

    Ok(())
}

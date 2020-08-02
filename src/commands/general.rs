use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, Utc};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage, EditMessage},
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, ModelError},
    prelude::Context,
    utils::Colour,
    Error, Result,
};

use crate::{models::caches::PictureCacheKey, utils::constants::*};

#[group]
#[commands(ping, invite, info, websites, peopleinspace, iss, exoplanet)]
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
                **Library:** [Serenity](https://github.com/serenity-rs/serenity)
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
                    a.name("Some websites with more information")
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

#[command]
fn exoplanet(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let mut res_msg: Message =
        msg.channel_id
            .send_message(&ctx.http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Exoplanet/Star Information").icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .description("<a:typing:393848431413559296> loading data...")
                })
            })?;

    let search_name = args.current().map(|s| s.to_owned());

    match ctx.data.read().get::<PictureCacheKey>() {
        None => return Err("can't get picture cache".into()),
        Some(p)
            if search_name.is_some() && p.host_stars.contains(&search_name.clone().unwrap()) =>
        {
            get_star(&ctx, &mut res_msg, &search_name.clone().unwrap())?
        }
        Some(p)
            if search_name.is_some() && p.exoplanets.contains(&search_name.clone().unwrap()) =>
        {
            get_planet(&ctx, &mut res_msg, &search_name.clone().unwrap())?
        }
        Some(_) if search_name.is_some() => {
            res_msg.edit(&ctx.http, |m: &mut EditMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    e.description(
                        "The name you gave isn't in the NASA Exoplanet Archive <:kia:367734893796655113>
                        Please understand that NASA has a 'weird' way of naming the stars in their archive
                        Here is a link to the list of all the stars in the archive: \
                        [star list](https://exoplanetarchive.ipac.caltech.edu/cgi-bin/nstedAPI/nph-nstedAPI?table=exoplanets&format=json&select=pl_hostname)"
                    )
                    .title("planet/star not found!")
                    .color(Colour::RED)
                })
            })?;
            return Ok(());
        }
        Some(p) => get_planet(
            &ctx,
            &mut res_msg,
            p.exoplanets
                .choose(&mut thread_rng())
                .ok_or("something went wrong while picking a planet")?,
        )?,
    };

    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
struct StarInfo {
    pub st_dist: Option<f64>,
    pub st_spstr: Option<String>,
    pub st_dens: Option<f64>,
    pub hd_name: Option<String>,
    pub st_age: Option<f64>,
    pub st_mass: Option<f64>,
    pub st_rad: Option<f64>,
    pub pl_num: i32,
    pub pl_letter: String,
}

impl StarInfo {
    fn get_lightyears_dist(&self) -> String {
        match &self.st_dist {
            Some(dist) => format!("{} lightyears", dist * 3.26156),
            None => "Unknown".to_owned(),
        }
    }

    fn get_age(&self) -> String {
        match &self.st_age {
            Some(age) => format!("{}Gyr (billion years)", age),
            None => "Unknown".to_owned(),
        }
    }

    fn get_mass(&self) -> String {
        match &self.st_mass {
            Some(mass) => format!("{} times the mass of the sun", mass),
            None => "Unknown".to_owned(),
        }
    }

    fn get_rad(&self) -> String {
        match &self.st_rad {
            Some(rad) => format!("{}R☉", rad),
            None => "Unknown".to_owned(),
        }
    }
}

fn get_star(ctx: &Context, msg: &mut Message, star_name: &str) -> CommandResult {
    let mut params = HashMap::new();
    params.insert("table", "exoplanets".to_owned());
    params.insert("format", "json".to_owned());
    params.insert("select", "st_dens,hd_name,pl_hostname,pl_letter,st_spstr,st_age,st_lum,st_mass,pl_pnum,st_rad,st_dist".to_owned());
    params.insert("where", format!("pl_hostname like '{}'", &star_name));

    let res: Vec<StarInfo> = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/cgi-bin/nstedAPI/nph-nstedAPI")
        .query(&params)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())?;

    let planets = res
        .iter()
        .map(|s| s.pl_letter.clone())
        .collect::<Vec<String>>()
        .join(", ");
    let star = &res[0];

    msg.edit(&ctx.http, |m: &mut EditMessage| {
        m.embed(|e: &mut CreateEmbed| {
            e.author(|a: &mut CreateEmbedAuthor| a.name("Star Information").icon_url(DEFAULT_ICON))
                .color(DEFAULT_COLOR)
                .title(star_name)
                .timestamp(&Utc::now())
                .field(
                    "System Data",
                    format!(
                        "**Number of planets in system:** {}
                        **Letters used to designate planets in the system:** {}
                        **Distance from us in lightyears:** {}
                        **Distance from us in parsecs:** {}",
                        star.pl_num,
                        planets,
                        star.get_lightyears_dist(),
                        star.st_dist
                            .map(|n| n.to_string())
                            .unwrap_or("unknown".to_owned()),
                    ),
                    false,
                )
                .field(
                    "Star Data",
                    format!(
                        "**Stellar Age:** {}
                        **Spectral Type:** {}
                        **Henry Draper Catalog Name:** {}
                        **Radius Star:** {}
                        **Mass of the star:** {}
                        **Stellar Density:** {}",
                        star.get_age(),
                        star.st_spstr
                            .as_ref()
                            .cloned()
                            .unwrap_or("unknown".to_owned()),
                        star.hd_name
                            .as_ref()
                            .cloned()
                            .unwrap_or("unknown".to_owned()),
                        star.get_rad(),
                        star.get_mass(),
                        star.st_dens
                            .map(|n| n.to_string())
                            .unwrap_or("unknown".to_owned()),
                    ),
                    false,
                )
        })
    })?;

    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
struct PlanetInfo {
    pub pl_masse: Option<f64>,
    pub pl_massj: Option<f64>,
    pub pl_eqt: Option<i32>,
    pub pl_telescope: Option<String>,
    pub pl_locale: Option<String>,
    pub pl_rade: Option<f64>,
    pub pl_radj: Option<f64>,
    pub pl_dens: Option<f64>,
    pub pl_orbeccen: Option<f64>,
    pub pl_orbincl: Option<f64>,
    pub pl_orbper: Option<f64>,
    pub pl_hostname: Option<String>,
    pub pl_orbsmax: Option<f64>,
    pub pl_disc: Option<i32>,
    pub pl_discmethod: Option<String>,
}

fn get_planet(ctx: &Context, msg: &mut Message, planet_name: &str) -> CommandResult {
    let mut params = HashMap::new();
    params.insert("table", "exoplanets".to_owned());
    params.insert("format", "json".to_owned());
    params.insert("select", "pl_masse,pl_massj,pl_eqt,pl_telescope,pl_locale,pl_rade,pl_radj,pl_dens,pl_orbeccen,pl_orbincl,pl_orbper,pl_hostname,pl_orbsmax,pl_disc,pl_discmethod".to_owned());
    params.insert("where", format!("pl_name like '{}'", &planet_name));

    let planet: PlanetInfo = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/cgi-bin/nstedAPI/nph-nstedAPI")
        .query(&params)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json::<Vec<PlanetInfo>>())
        .map(|r| r[0].clone())?;

    msg.edit(&ctx.http, |m: &mut EditMessage| {
        m.embed(|e: &mut CreateEmbed| {
            e.author(|a: &mut CreateEmbedAuthor| {
                a.name("Planet Information").icon_url(DEFAULT_ICON)
            })
            .color(DEFAULT_COLOR)
            .title(planet_name)
            .timestamp(&Utc::now())
            .field(
                "Planet Data",
                format!(
                    "**Planet Radius compared to Jupiter:** {}
                    **Planet Radius compared to Earth:** {}
                    **Planet Density:** {}
                    **Planet Mass compared to Jupiter:** {}
                    **Planet Mass compared to Earth:** {}
                    **Planet Equilibrium Temperature:** {}",
                    planet
                        .pl_radj
                        .map(|n| format!("{} times", n))
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_rade
                        .map(|n| format!("{} times", n))
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_dens
                        .map(|n| format!("{}g/cm³", n))
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_massj
                        .map(|n| format!("{} times", n))
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_masse
                        .map(|n| format!("{} times", n))
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_eqt
                        .map(|n| format!("{}K", n))
                        .unwrap_or("unknown".to_owned()),
                ),
                false,
            )
            .field(
                "Orbit Data",
                format!(
                    "**Eccentricity:** {}
                    **Inclination:** {}
                    **Orbital Period:** {} days
                    **Orbit Semi-Major Axis:** {}
                    **Host Star:** {}",
                    planet
                        .pl_orbeccen
                        .map(|n| n.to_string())
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_orbincl
                        .map(|n| format!("{} degrees", n))
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_orbper
                        .map(|n| format!("{}K", n))
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_orbsmax
                        .map(|n| format!("{}AU", n))
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_hostname
                        .as_ref()
                        .cloned()
                        .unwrap_or("unknown".to_owned()),
                ),
                false,
            )
            .field(
                "Discovery Info",
                format!(
                    "**Year of Discovery:** {}
                    **Discovery Method:** {}
                    **Location of observation of planet discovery:** {}
                    **Name of telescoped used:** {}",
                    planet
                        .pl_disc
                        .map(|n| n.to_string())
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_discmethod
                        .as_ref()
                        .cloned()
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_locale
                        .as_ref()
                        .cloned()
                        .unwrap_or("unknown".to_owned()),
                    planet
                        .pl_telescope
                        .as_ref()
                        .cloned()
                        .unwrap_or("unknown".to_owned()),
                ),
                false,
            )
        })
    })?;

    Ok(())
}

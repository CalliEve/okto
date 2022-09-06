use std::collections::HashMap;

use chrono::Utc;
use itertools::Itertools;
use okto_framework::macros::command;
use rand::seq::SliceRandom;
use serde::{
    Deserialize,
    Serialize,
};
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
        CreateEmbedFooter,
        CreateInteractionResponse,
        EditInteractionResponse,
    },
    framework::standard::CommandResult,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction,
        InteractionResponseType,
    },
    prelude::Context,
    utils::Colour,
};

use crate::{
    models::caches::PictureCacheKey,
    utils::constants::*,
};

#[command]
/// Get the ping of the bot
async fn ping(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    let start = Utc::now();
    interaction
        .create_interaction_response(
            &ctx,
            |c: &mut CreateInteractionResponse| {
                c.interaction_response_data(|d| {
                    d.embed(|e: &mut CreateEmbed| e.description("\u{1f3d3} pong..."))
                })
            },
        )
        .await?;
    let end = Utc::now();

    let round_trip = end - start;
    let ws_delay = start
        - *interaction
            .id
            .created_at();

    interaction
        .edit_original_interaction_response(
            ctx,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.title("Pong!")
                        .description(format!(
                            "\u{1f3d3}\nws delay: {}ms\napi ping: {}ms",
                            ws_delay.num_milliseconds(),
                            round_trip.num_milliseconds()
                        ))
                })
            },
        )
        .await?;

    Ok(())
}

#[command]
/// Get some general information about the bot
async fn info(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    let user_id = ctx
        .cache
        .current_user()
        .id;
    interaction.create_interaction_response(&ctx.http, |m: &mut CreateInteractionResponse| {
            m.interaction_response_data(|c| {c.embed(|e: &mut CreateEmbed| {
            e.title("OKTO")
            .description(
                format!(
                    "This is a bot to show upcoming launches and provide additional information on everything to do with spaceflight\n\
                    **Author:** Calli#3141\n\
                    **Version:** `4.0` \"slash-commands\"\n\
                    **Source Code:** [GitHub link](https://github.com/callieve/okto)\n\
                    **Library:** [Serenity](https://github.com/serenity-rs/serenity)\n\
                    **Total servers:** {}\n\
                    <:RustRainbow:752508751675654204>\n\
                    \n<:discord:314003252830011395>\n\
                    [**Support Server**](https://discord.gg/dXPHfPJ)\n\
                    [**The Space Devs**](https://discord.gg/p7ntkNA)\n\
                    [**Rocket Watch server**](https://discord.gg/Hyd4umg)\n\
                    \n<:botTag:230105988211015680>\n\
                    If you want OKTO on your server, click [**here**](https://discord.com/api/oauth2/authorize?client_id={}&permissions=388160&scope=bot%20applications.commands)\n\
                    If you like OKTO, please [**vote**](https://discordbots.org/bot/429306620439166977/vote) ^-^",
                    ctx.cache.guild_count(), user_id
                )
            )
            .author(|a: &mut CreateEmbedAuthor| {
                a.name("Bot Information")
                .icon_url(DEFAULT_ICON)
            })
            .thumbnail(DEFAULT_ICON)
            .color(DEFAULT_COLOR)
        })})
    }).await?;
    Ok(())
}

#[command]
/// Have some helpful websites
async fn websites(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(
            &ctx.http,
            |m: &mut CreateInteractionResponse| {
                m.interaction_response_data(|c| {
                    c.embed(|e: &mut CreateEmbed| {
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
                })
            },
        )
        .await?;
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
/// Get a list of all humans currently in space
async fn peopleinspace(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> CommandResult {
    let pis: PeopleInSpaceResp = DEFAULT_CLIENT
        .get("http://api.open-notify.org/astros.json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut text_vec: Vec<String> = Vec::with_capacity(
        pis.people
            .len(),
    );
    for person in &pis.people {
        text_vec.push(format!(
            "{}: {}\n",
            person.name, person.craft
        ));
    }

    interaction
        .create_interaction_response(
            &ctx.http,
            |m: &mut CreateInteractionResponse| {
                m.interaction_response_data(|c| {
                    c.embed(|e: &mut CreateEmbed| {
                        e.title(format!(
                            "There are currently {} people in space",
                            pis.number
                        ))
                        .description(
                            text_vec
                                .iter()
                                .map(std::string::String::as_str)
                                .collect::<String>(),
                        )
                        .author(|a: &mut CreateEmbedAuthor| {
                            a.name("People in space")
                                .icon_url(DEFAULT_ICON)
                        })
                        .timestamp(Utc::now())
                        .color(DEFAULT_COLOR)
                    })
                })
            },
        )
        .await?;

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
/// Get the current location of the ISS
async fn iss(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(&ctx.http, |c| {
            c.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await?;

    let iss_pos: ISSLocation = DEFAULT_CLIENT
        .get("https://api.wheretheiss.at/v1/satellites/25544")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

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

    interaction.edit_original_interaction_response(&ctx.http, |e: &mut EditInteractionResponse| {
        e.embed(|e: &mut CreateEmbed| {
            e.description(format!(
                "**Latitude:** {0:.5}\n**Longitude:** {1:.5}\n**Altitude:** {2:.3}km\n**Velocity:** {3:.3}km/h",
                iss_pos.latitude, iss_pos.longitude, iss_pos.altitude, iss_pos.velocity
            ))
            .author(|a: &mut CreateEmbedAuthor| a.name("Position ISS").icon_url(DEFAULT_ICON))
            .image(detail_url)
            .thumbnail(global_url)
            .footer(|f: &mut CreateEmbedFooter| f.text("source: wheretheiss.at"))
            .timestamp(Utc::now())
            .color(DEFAULT_COLOR)
        })
    })
    .await?;

    Ok(())
}

#[command]
#[options (
        {
            option_type: String,
            name: "exoplanet",
            description: "Name of the exoplanet to search for",
            required: false
        },
        {
            option_type: String,
            name: "star",
            description: "Name of the star to search for",
            required: false
        }
)]
/// Get information about an exoplanet or star
async fn exoplanet(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    interaction
        .create_interaction_response(&ctx.http, |c| {
            c.kind(InteractionResponseType::DeferredChannelMessageWithSource)
        })
        .await?;

    let planet_name: Option<String> = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "exoplanet")
        .and_then(|o| {
            o.value
                .clone()
        })
        .and_then(|v| {
            v.as_str()
                .map(ToOwned::to_owned)
        });
    let star_name = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "star")
        .and_then(|o| {
            o.value
                .clone()
        })
        .and_then(|v| {
            v.as_str()
                .map(ToOwned::to_owned)
        });

    match ctx
        .data
        .read()
        .await
        .get::<PictureCacheKey>()
    {
        None => return Err("can't get picture cache".into()),
        Some(p)
            if star_name
                .clone()
                .filter(|name| {
                    p.host_stars
                        .contains(name)
                })
                .is_some() =>
        {
            get_star(
                ctx,
                interaction,
                &star_name
                    .clone()
                    .unwrap(),
            )
            .await?
        },
        Some(p)
            if planet_name
                .clone()
                .filter(|name| {
                    p.exoplanets
                        .contains(name)
                })
                .is_some() =>
        {
            get_planet(
                ctx,
                interaction,
                &planet_name
                    .clone()
                    .unwrap(),
            )
            .await?
        },
        Some(_) if planet_name.is_some() || star_name.is_some() => {
            interaction.edit_original_interaction_response(&ctx.http, |e: &mut EditInteractionResponse| {
        e.embed(|e: &mut CreateEmbed| {
                    e.description(
                        "The name you gave isn't in the NASA Exoplanet Archive <:kia:367734893796655113>
                        Please understand that NASA has a 'weird' way of naming the stars in their archive
                        Here is a link to the list of all the stars in the archive: \
                        [star list](https://exoplanetarchive.ipac.caltech.edu/TAP/sync?format=csv&query=select%20hostname%20from%20ps"
                    )
                    .title("planet/star not found!")
                    .color(Colour::RED)
                })
            }).await?;
            return Ok(());
        },
        Some(p) => {
            let rand_name = {
                p.exoplanets
                    .choose(
                        &mut *RNG
                            .lock()
                            .await,
                    )
                    .ok_or("something went wrong while picking a planet")
            }?;
            get_planet(ctx, interaction, rand_name).await?
        },
    };

    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
struct StarInfo {
    pub sy_dist: Option<f64>,
    pub st_spectype: Option<String>,
    pub st_dens: Option<f64>,
    pub hd_name: Option<String>,
    pub st_age: Option<f64>,
    pub st_mass: Option<f64>,
    pub st_rad: Option<f64>,
    pub sy_pnum: i32,
    pub pl_letter: String,
    pub hostname: String,
}

impl StarInfo {
    fn get_lightyears_dist(&self) -> String {
        match &self.sy_dist {
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
            Some(rad) => format!("{}R\u{2609}", rad),
            None => "Unknown".to_owned(),
        }
    }
}

async fn get_star(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    star_name: &str,
) -> CommandResult {
    let mut params = HashMap::new();
    params.insert("format", "json".to_owned());
    params.insert("query", format!("select st_dens,hd_name,hostname,pl_letter,st_spectype,st_age,st_lum,st_mass,sy_pnum,st_rad,sy_dist from ps where hostname = '{}'", &star_name));

    let res: Vec<StarInfo> = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/TAP/sync")
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<StarInfo>>()
        .await?
        .into_iter()
        .unique_by(|s| {
            s.hostname
                .clone()
        })
        .collect();

    let planets = res
        .iter()
        .map(|s| {
            s.pl_letter
                .clone()
        })
        .unique()
        .collect::<Vec<String>>()
        .join(", ");
    let star = &res[0];

    interaction
        .edit_original_interaction_response(
            &ctx.http,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Star Information")
                            .icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .title(star_name)
                    .timestamp(Utc::now())
                    .field(
                        "System Data",
                        format!(
                            "**Number of planets in system:** {}\n\
                        **Letters used to designate planets in the system:** {}\n\
                        **Distance from us in lightyears:** {}\n\
                        **Distance from us in parsecs:** {}",
                            star.sy_pnum,
                            planets,
                            star.get_lightyears_dist(),
                            star.sy_dist
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| n.to_string()
                                ),
                        ),
                        false,
                    )
                    .field(
                        "Star Data",
                        format!(
                            "**Stellar Age:** {}\n\
                        **Spectral Type:** {}\n\
                        **Henry Draper Catalog Name:** {}\n\
                        **Radius Star:** {}\n\
                        **Mass of the star:** {}\n\
                        **Stellar Density:** {}",
                            star.get_age(),
                            star.st_spectype
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_owned()),
                            star.hd_name
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_owned()),
                            star.get_rad(),
                            star.get_mass(),
                            star.st_dens
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| n.to_string()
                                ),
                        ),
                        false,
                    )
                })
            },
        )
        .await?;

    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
struct PlanetInfo {
    pub pl_masse: Option<f64>,
    pub pl_massj: Option<f64>,
    pub pl_eqt: Option<f64>,
    pub disc_telescope: Option<String>,
    pub disc_locale: Option<String>,
    pub pl_rade: Option<f64>,
    pub pl_radj: Option<f64>,
    pub pl_dens: Option<f64>,
    pub pl_orbeccen: Option<f64>,
    pub pl_orbincl: Option<f64>,
    pub pl_orbper: Option<f64>,
    pub hostname: Option<String>,
    pub pl_orbsmax: Option<f64>,
    pub disc_year: Option<i32>,
    pub discoverymethod: Option<String>,
    pub pl_name: String,
}

async fn get_planet(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    planet_name: &str,
) -> CommandResult {
    let mut params = HashMap::new();
    params.insert("format", "json".to_owned());
    params.insert("query", format!("select pl_name,pl_masse,pl_massj,pl_eqt,disc_telescope,disc_locale,pl_rade,pl_radj,pl_dens,pl_orbeccen,pl_orbincl,pl_orbper,hostname,pl_orbsmax,disc_year,discoverymethod from ps where pl_name = '{}'", &planet_name));

    let planet: PlanetInfo = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/TAP/sync")
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<PlanetInfo>>()
        .await?
        .into_iter()
        .unique_by(|p| {
            p.pl_name
                .clone()
        })
        .next()
        .ok_or("No planet like this found")?;

    interaction
        .edit_original_interaction_response(
            &ctx.http,
            |e: &mut EditInteractionResponse| {
                e.embed(|e: &mut CreateEmbed| {
                    e.author(|a: &mut CreateEmbedAuthor| {
                        a.name("Planet Information")
                            .icon_url(DEFAULT_ICON)
                    })
                    .color(DEFAULT_COLOR)
                    .title(planet_name)
                    .timestamp(Utc::now())
                    .field(
                        "Planet Data",
                        format!(
                            "**Planet Radius compared to Jupiter:** {}\n\
                    **Planet Radius compared to Earth:** {}\n\
                    **Planet Density:** {}\n\
                    **Planet Mass compared to Jupiter:** {}\n\
                    **Planet Mass compared to Earth:** {}\n\
                    **Planet Equilibrium Temperature:** {}",
                            planet
                                .pl_radj
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{} times", n)
                                ),
                            planet
                                .pl_rade
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{} times", n)
                                ),
                            planet
                                .pl_dens
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{}g/cm\u{b3}", n)
                                ),
                            planet
                                .pl_massj
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{} times", n)
                                ),
                            planet
                                .pl_masse
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{} times", n)
                                ),
                            planet
                                .pl_eqt
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{}K", n)
                                ),
                        ),
                        false,
                    )
                    .field(
                        "Orbit Data",
                        format!(
                            "**Eccentricity:** {}\n\
                    **Inclination:** {}\n\
                    **Orbital Period:** {} days\n\
                    **Orbit Semi-Major Axis:** {}\n\
                    **Host Star:** {}",
                            planet
                                .pl_orbeccen
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| n.to_string()
                                ),
                            planet
                                .pl_orbincl
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{} degrees", n)
                                ),
                            planet
                                .pl_orbper
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{}K", n)
                                ),
                            planet
                                .pl_orbsmax
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| format!("{}AU", n)
                                ),
                            planet
                                .hostname
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_owned()),
                        ),
                        false,
                    )
                    .field(
                        "Discovery Info",
                        format!(
                            "**Year of Discovery:** {}\n\
                    **Discovery Method:** {}\n\
                    **Location of observation of planet discovery:** {}\n\
                    **Name of telescoped used:** {}",
                            planet
                                .disc_year
                                .map_or_else(
                                    || "unknown".to_owned(),
                                    |n| n.to_string()
                                ),
                            planet
                                .discoverymethod
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_owned()),
                            planet
                                .disc_locale
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_owned()),
                            planet
                                .disc_telescope
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_owned()),
                        ),
                        false,
                    )
                })
            },
        )
        .await?;

    Ok(())
}

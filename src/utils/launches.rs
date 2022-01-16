use std::{
    collections::HashMap,
    str::FromStr,
};

use reqwest::header::AUTHORIZATION;
use serenity::{framework::standard::CommandError, model::{interactions::{application_command::ApplicationCommandInteraction, message_component::ButtonStyle}, channel::ReactionType}};

use crate::{
    models::launches::{
        LaunchData,
        LaunchInfo,
        VidURL,
    },
    utils::constants::{
        DEFAULT_CLIENT,
        LAUNCH_AGENCIES,
        LAUNCH_VEHICLES,
        LL_KEY,
    }, events::statefulembed::ButtonType,
};

use super::constants::{LAST_PAGE_EMOJI, NEXT_PAGE_EMOJI, FIRST_PAGE_EMOJI, FINAL_PAGE_EMOJI, EXIT_EMOJI};

pub fn format_links(links: &[VidURL]) -> Option<String> {
    let mut res = String::new();

    for link_obj in links {
        if let Ok(link) = url::Url::from_str(&link_obj.url) {
            if let Some(mut domain) = link.domain() {
                domain = domain.trim_start_matches("www.");

                res.push_str(&format!(
                    "\"{}\"\n[{}]({})\n\n",
                    link_obj
                        .title
                        .as_ref()
                        .map_or("unknown url", String::as_str),
                    domain,
                    &link_obj.url
                ));
            }
        }
    }

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}

pub fn filter_launches(launches: Vec<LaunchData>, interaction: &ApplicationCommandInteraction) -> Result<Vec<LaunchData>, String> {
    let agency_filter = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "lsp")
        .and_then(|o| {
            o.value
                .clone()
        })
        .and_then(|v| {
            v.as_str().map(|s| s.to_owned())
        });

    if let Some(lsp) = agency_filter {
    if let Some(filter) = LAUNCH_AGENCIES.get(&lsp.as_str()) {
        let filtered = launches
            .into_iter()
            .filter(|l| l.lsp == *filter)
            .collect::<Vec<LaunchData>>();
        if filtered.is_empty() {
            return Err("this launch provider does not have any upcoming launches :(".to_owned());
        }
        return Ok(filtered);
    }
    }
    
    let rocket_filter = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "rocket")
        .and_then(|o| {
            o.value
                .clone()
        })
        .and_then(|v| {
            v.as_str().map(|s| s.to_owned())
        });

    if let Some(rocket) = rocket_filter {
    if let Some(filter) = LAUNCH_VEHICLES.get(rocket.as_str()) {
        let filtered = launches
            .into_iter()
            .filter(|l| {
                filter.contains(
                    &l.vehicle
                        .as_str(),
                )
            })
            .collect::<Vec<LaunchData>>();
        if filtered.is_empty() {
            return Err("this launch vehicle does not have any upcoming launches :(".to_owned());
        }
        return Ok(filtered);
    }
    }

    Ok(launches)
}

pub async fn request_launch(id: &str) -> Result<LaunchData, CommandError> {
    let mut params = HashMap::new();
    params.insert("mode", "detailed");

    let res: LaunchInfo = DEFAULT_CLIENT
        .get(&format!("https://ll.thespacedevs.com/2.0.0/launch/{}/", id))
        .header(AUTHORIZATION, LL_KEY.as_str())
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(res.into())
}

#[derive(Debug, Clone, Copy)]
pub enum StandardButton {
    First,
    Last,
    Forward,
    Back,
    Exit
}

impl StandardButton {
    pub fn to_button(&self) -> ButtonType {
        match *self {
            Self::Last => ButtonType {
                label: "Last page".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(ReactionType::from(FINAL_PAGE_EMOJI))
            },
            Self::First => ButtonType {
                label: "First page".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(ReactionType::from(FIRST_PAGE_EMOJI))
            },
            Self::Forward => ButtonType {
                label: "Forward one page".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(ReactionType::from(NEXT_PAGE_EMOJI))
            },
            Self::Back => ButtonType {
                label: "Back one page".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(ReactionType::from(LAST_PAGE_EMOJI))
            },
            Self::Exit => ButtonType {
                label: "Exit".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(EXIT_EMOJI))
            }
        }
    }
}

use std::{
    fmt::{
        self,
        Display,
        Write,
    },
    str::FromStr,
};

use serenity::model::application::CommandInteraction;

use crate::{
    models::launches::{
        LaunchData,
        VidURL,
    },
    utils::constants::{
        LAUNCH_AGENCIES,
        LAUNCH_VEHICLES,
    },
};

pub fn format_links(links: &[VidURL]) -> Option<String> {
    let mut res = String::new();

    for link_obj in links {
        if let Ok(link) = url::Url::from_str(&link_obj.url) {
            if let Some(mut domain) = link.domain() {
                domain = domain.trim_start_matches("www.");

                write!(
                    res,
                    "\"{}\"\n[{}]({})\n\n",
                    link_obj
                        .title
                        .as_ref()
                        .map_or("unknown url", String::as_str),
                    domain,
                    &link_obj.url
                )
                .expect("write to String: can't fail");
            }
        }
    }

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}

pub fn filter_launches(
    launches: Vec<LaunchData>,
    interaction: &CommandInteraction,
) -> Result<Vec<LaunchData>, FilterErrorType> {
    let agency_filter = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "lsp")
        .and_then(|o| {
            o.value
                .as_str()
        })
        .map(str::to_lowercase);

    if let Some(lsp) = agency_filter {
        if let Some(filter) = LAUNCH_AGENCIES.get(&lsp.as_str()) {
            let filtered = launches
                .into_iter()
                .filter(|l| l.lsp == *filter)
                .collect::<Vec<LaunchData>>();
            if filtered.is_empty() {
                return Err(FilterErrorType::Lsp);
            }
            return Ok(filtered);
        }

        return Err(FilterErrorType::Invalid);
    }

    let rocket_filter = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "rocket")
        .and_then(|o| {
            o.value
                .as_str()
        })
        .map(ToOwned::to_owned);

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
                return Err(FilterErrorType::Vehicle);
            }
            return Ok(filtered);
        }

        return Err(FilterErrorType::Invalid);
    }

    Ok(launches)
}

#[derive(Debug, Clone, Copy)]
pub enum FilterErrorType {
    Vehicle,
    Lsp,
    Invalid,
}

impl Display for FilterErrorType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Self::Vehicle => fmt.write_str("launch vehicle"),
            Self::Lsp => fmt.write_str("launch provider"),
            Self::Invalid => fmt.write_str("invalid"),
        }
    }
}

use std::str::FromStr;

use serenity::model::interactions::application_command::ApplicationCommandInteraction;

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

pub fn filter_launches(
    launches: Vec<LaunchData>,
    interaction: &ApplicationCommandInteraction,
) -> Result<Vec<LaunchData>, String> {
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
            v.as_str()
                .map(|s| s.to_owned())
        });

    if let Some(lsp) = agency_filter {
        if let Some(filter) = LAUNCH_AGENCIES.get(&lsp.as_str()) {
            let filtered = launches
                .into_iter()
                .filter(|l| l.lsp == *filter)
                .collect::<Vec<LaunchData>>();
            if filtered.is_empty() {
                return Err(
                    "this launch provider does not have any upcoming launches :(".to_owned(),
                );
            }
            return Ok(filtered);
        }

        return Err(
            "This is not a valid filter, please take a look at those listed in `/filtersinfo`"
                .to_owned(),
        );
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
            v.as_str()
                .map(|s| s.to_owned())
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

        return Err(
            "This is not a valid filter, please take a look at those listed in `/filtersinfo`"
                .to_owned(),
        );
    }

    Ok(launches)
}

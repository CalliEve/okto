use std::{collections::HashMap, str::FromStr};

use reqwest::header::AUTHORIZATION;
use serenity::framework::standard::{Args, CommandError};

use crate::{
    models::launches::{LaunchData, LaunchInfo, VidURL},
    utils::constants::{DEFAULT_CLIENT, LAUNCH_AGENCIES, LAUNCH_VEHICLES, LL_KEY},
};

pub fn format_links(links: &[VidURL]) -> Option<String> {
    let mut res = String::new();

    for link_obj in links {
        if let Ok(link) = url::Url::from_str(&link_obj.url) {
            if let Some(mut domain) = link.domain() {
                domain = domain.trim_start_matches("www.");

                res.push_str(&format!(
                    "\"{}\"\n[{}]({})\n\n",
                    &link_obj.title, domain, &link_obj.url
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

pub fn filter_launches(launches: Vec<LaunchData>, args: Args) -> Result<Vec<LaunchData>, String> {
    let filter_arg = if let Some(f) = args.remains() {
        f.to_lowercase()
    } else {
        return Ok(launches);
    };

    if let Some(filter) = LAUNCH_AGENCIES.get(&filter_arg.as_str()) {
        let filtered = launches
            .into_iter()
            .filter(|l| l.lsp == *filter)
            .collect::<Vec<LaunchData>>();
        if filtered.is_empty() {
            return Err("this launch provider does not have any upcoming launches :(".to_owned());
        }
        return Ok(filtered);
    }

    if let Some(filter) = LAUNCH_VEHICLES.get(&filter_arg.as_str()) {
        let filtered = launches
            .into_iter()
            .filter(|l| filter.contains(&l.vehicle.as_str()))
            .collect::<Vec<LaunchData>>();
        if filtered.is_empty() {
            return Err("this launch vehicle does not have any upcoming launches :(".to_owned());
        }
        return Ok(filtered);
    }

    Ok(launches)
}

pub fn request_launch(id: &str) -> Result<LaunchData, CommandError> {
    let mut params = HashMap::new();
    params.insert("mode", "detailed");

    let res: LaunchInfo = DEFAULT_CLIENT
        .get(&format!("https://ll.thespacedevs.com/2.0.0/launch/{}", id))
        .header(AUTHORIZATION, LL_KEY.as_str())
        .query(&params)
        .send()?
        .error_for_status()?
        .json()?;
    Ok(res.into())
}

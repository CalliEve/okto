use std::{collections::HashMap, str::FromStr};

use reqwest::header::AUTHORIZATION;
use serenity::framework::standard::{Args, CommandError};

use crate::{
    models::launches::{LaunchData, LaunchInfo},
    utils::constants::{DEFAULT_CLIENT, LAUNCH_AGENCIES, LAUNCH_VEHICLES, LL_KEY},
};

pub fn format_links(links: &[String]) -> Option<String> {
    let mut res = String::new();

    for str_link in links {
        if let Ok(link) = url::Url::from_str(&str_link) {
            if let Some(domain) = link.domain() {
                res.push_str(&format!("[{}]({})", domain, &str_link));
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
    params.insert(
        "fields",
        "vidURLs,status,name,rocket,lsp,net,location,tbddate,tbdtime,windowstart,windowend,missions,mission",
    );

    let res: LaunchInfo = DEFAULT_CLIENT
        .get(&format!("https://ll.thespacedevs.com/2.0.0/launch/{}", id))
        .header(AUTHORIZATION, LL_KEY.as_str())
        .query(&params)
        .send()?
        .error_for_status()?
        .json()?;
    Ok(res.into())
}

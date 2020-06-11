use std::str::FromStr;

use serenity::framework::standard::Args;

use crate::{
    models::launches::LaunchData,
    utils::constants::{LAUNCH_AGENCIES, LAUNCH_VEHICLES},
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
        f
    } else {
        return Ok(launches);
    };

    if let Some(filter) = LAUNCH_AGENCIES.get(&filter_arg) {
        let filtered = launches
            .into_iter()
            .filter(|l| l.lsp == *filter)
            .collect::<Vec<LaunchData>>();
        if filtered.is_empty() {
            return Err("this launch provider does not have any upcoming launches :(".to_owned());
        }
        return Ok(filtered);
    }

    if let Some(filter) = LAUNCH_VEHICLES.get(&filter_arg) {
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

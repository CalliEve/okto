use serde::Deserialize;
use std::{collections::HashMap, time::Duration};

use super::constants::*;
use crate::models::{caches::PictureDataCache, pictures::MarsRoverPicture};

#[derive(Deserialize, Debug, Clone)]
struct HubbleIDContainer {
    pub id: i32,
}

#[derive(Deserialize, Debug, Clone)]
struct CuriosityContainer {
    pub photos: Vec<MarsRoverPicture>,
}

#[derive(Deserialize, Debug, Clone)]
struct ExoplanetContainer {
    pub pl_name: String,
}

#[derive(Deserialize, Debug, Clone)]
struct HostStarContainer {
    pub pl_hostname: String,
}

fn hubble_pics() -> Vec<i32> {
    let hubble_res: Vec<HubbleIDContainer> = DEFAULT_CLIENT
        .get("http://hubblesite.org/api/v3/images?collection=news&page=all")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .unwrap_or_else(|e| {
            dbg!(e);
            Vec::new()
        });
    hubble_res.iter().map(|h| h.id).collect()
}

fn curiosity_mardi() -> Vec<MarsRoverPicture> {
    let curiosity_res: CuriosityContainer = DEFAULT_CLIENT
        .get(format!(
            "https://api.nasa.gov/mars-photos/api/v1/rovers/curiosity/photos?camera=mardi&sol=0&api_key={}",
            NASA_KEY.as_str()
        ).as_str())
        .timeout(Duration::from_secs(40))
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .unwrap_or_else(|e| {
            dbg!(e);
            CuriosityContainer {
                photos: Vec::new()
            }
        });

    if curiosity_res.photos.is_empty() {
        Vec::new()
    } else {
        curiosity_res.photos[1000..3659].to_vec()
    }
}

fn exoplanets() -> Vec<String> {
    let mut params = HashMap::new();
    params.insert("table", "exoplanets");
    params.insert("format", "json");
    params.insert("select", "pl_name");

    let exoplanet_res: Vec<ExoplanetContainer> = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/cgi-bin/nstedAPI/nph-nstedAPI")
        .query(&params)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .unwrap_or_else(|e| {
            dbg!(e);
            Vec::new()
        });

    exoplanet_res.into_iter().map(|h| h.pl_name).collect()
}

fn host_stars() -> Vec<String> {
    let mut params = HashMap::new();
    params.insert("table", "exoplanets");
    params.insert("format", "json");
    params.insert("select", "pl_hostname");

    let host_star_res: Vec<HostStarContainer> = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/cgi-bin/nstedAPI/nph-nstedAPI")
        .query(&params)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .unwrap_or_else(|e| {
            dbg!(e);
            Vec::new()
        });

    host_star_res.into_iter().map(|h| h.pl_hostname).collect()
}

pub fn preload_data() -> PictureDataCache {
    PictureDataCache {
        hubble_pics: hubble_pics(),
        curiosity_mardi: curiosity_mardi(),
        exoplanets: exoplanets(),
        host_stars: host_stars(),
    }
}

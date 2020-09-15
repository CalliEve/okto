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

async fn hubble_pics() -> reqwest::Result<Vec<i32>> {
    let hubble_res: Vec<HubbleIDContainer> = DEFAULT_CLIENT
        .get("http://hubblesite.org/api/v3/images?collection=news&page=all")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(hubble_res.iter().map(|h| h.id).collect())
}

async fn curiosity_mardi() -> reqwest::Result<Vec<MarsRoverPicture>> {
    let curiosity_res: CuriosityContainer = DEFAULT_CLIENT
        .get(format!(
            "https://api.nasa.gov/mars-photos/api/v1/rovers/curiosity/photos?camera=mardi&sol=0&api_key={}",
            NASA_KEY.as_str()
        ).as_str())
        .timeout(Duration::from_secs(40))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(if curiosity_res.photos.is_empty() {
        Vec::new()
    } else {
        curiosity_res.photos[1000..3659].to_vec()
    })
}

async fn exoplanets() -> reqwest::Result<Vec<String>> {
    let mut params = HashMap::new();
    params.insert("table", "exoplanets");
    params.insert("format", "json");
    params.insert("select", "pl_name");

    let exoplanet_res: Vec<ExoplanetContainer> = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/cgi-bin/nstedAPI/nph-nstedAPI")
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(exoplanet_res.into_iter().map(|h| h.pl_name).collect())
}

async fn host_stars() -> reqwest::Result<Vec<String>> {
    let mut params = HashMap::new();
    params.insert("table", "exoplanets");
    params.insert("format", "json");
    params.insert("select", "pl_hostname");

    let host_star_res: Vec<HostStarContainer> = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/cgi-bin/nstedAPI/nph-nstedAPI")
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(host_star_res.into_iter().map(|h| h.pl_hostname).collect())
}

pub async fn preload_data() -> PictureDataCache {
    let (hubble_pics, curiosity_mardi, exoplanets, host_stars) =
        tokio::join!(hubble_pics(), curiosity_mardi(), exoplanets(), host_stars());

    PictureDataCache {
        hubble_pics: hubble_pics.unwrap_or_else(|e| {
            dbg!(e);
            Vec::new()
        }),
        curiosity_mardi: curiosity_mardi.unwrap_or_else(|e| {
            dbg!(e);
            Vec::new()
        }),
        exoplanets: exoplanets.unwrap_or_else(|e| {
            dbg!(e);
            Vec::new()
        }),
        host_stars: host_stars.unwrap_or_else(|e| {
            dbg!(e);
            Vec::new()
        }),
    }
}

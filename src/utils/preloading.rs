use std::{
    collections::HashMap,
    time::Duration,
};

use itertools::Itertools;
use serde::Deserialize;

use super::constants::*;
use crate::models::{
    caches::PictureDataCache,
    pictures::MarsRoverPicture,
};

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
    pub hostname: String,
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

    Ok(
        if curiosity_res
            .photos
            .is_empty()
        {
            Vec::new()
        } else {
            curiosity_res.photos[1000..3659].to_vec()
        },
    )
}

async fn exoplanets() -> reqwest::Result<Vec<String>> {
    let mut params = HashMap::new();
    params.insert("format", "json");
    params.insert("query", "select pl_name from ps");

    let exoplanet_res: Vec<ExoplanetContainer> = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/TAP/sync")
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(exoplanet_res
        .into_iter()
        .map(|h| h.pl_name)
        .unique()
        .collect())
}

async fn host_stars() -> reqwest::Result<Vec<String>> {
    let mut params = HashMap::new();
    params.insert("format", "json");
    params.insert("query", "select hostname from ps");

    let host_star_res: Vec<HostStarContainer> = DEFAULT_CLIENT
        .get("https://exoplanetarchive.ipac.caltech.edu/TAP/sync")
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(host_star_res
        .into_iter()
        .map(|h| h.hostname)
        .unique()
        .collect())
}

pub async fn preload_data() -> PictureDataCache {
    let (curiosity_mardi, exoplanets, host_stars) =
        tokio::join!(curiosity_mardi(), exoplanets(), host_stars());

    PictureDataCache {
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

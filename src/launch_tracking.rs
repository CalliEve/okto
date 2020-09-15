use reqwest::{header::AUTHORIZATION, Result};
use serenity::prelude::RwLock;
use std::{collections::HashMap, convert::TryFrom, sync::Arc};

use crate::{
    models::launches::{LaunchContainer, LaunchData},
    utils::constants::{DEFAULT_CLIENT, LL_KEY},
};

pub async fn launch_tracking(cache: Arc<RwLock<Vec<LaunchData>>>) {
    println!("getting launch information");

    let mut launches: Vec<LaunchData> = match get_new_launches().await {
        Ok(ls) => ls.results.into_iter().map(LaunchData::from).collect(),
        Err(e) => {
            dbg!(e);
            return;
        },
    };
    launches.sort_by_key(|l| l.net);

    for (i, launch) in launches.iter_mut().enumerate() {
        launch.id = if let Ok(id) = i32::try_from(i) {
            id
        } else {
            return;
        };
    }

    println!("got {} launches", launches.len());

    let mut launch_cache = cache.write().await;
    launch_cache.clear();
    launch_cache.append(&mut launches);
}

async fn get_new_launches() -> Result<LaunchContainer> {
    let mut params = HashMap::new();
    params.insert("limit", "100");
    params.insert("mode", "detailed");

    Ok(DEFAULT_CLIENT
        .get("https://ll.thespacedevs.com/2.0.0/launch/upcoming")
        .header(AUTHORIZATION, LL_KEY.as_str())
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

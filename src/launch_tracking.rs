use reqwest::{header::AUTHORIZATION, Result};
use serenity::prelude::RwLock;
use std::{collections::HashMap, sync::Arc};

use crate::{
    models::launches::{LaunchContainer, LaunchData},
    utils::constants::{DEFAULT_CLIENT, LL_KEY},
};

pub fn launch_tracking(cache: Arc<RwLock<Vec<LaunchData>>>) {
    println!("getting launch information");

    let mut launches: Vec<LaunchData> = match get_new_launches() {
        Ok(ls) => ls.results.into_iter().map(LaunchData::from).collect(),
        Err(e) => {
            dbg!(e);
            return;
        }
    };
    launches.sort_by_key(|l| l.net);

    let mut i = 0;
    for launch in launches.iter_mut() {
        launch.id = i;
        i += 1;
    }

    println!("got {} launches", launches.len());

    {
        let mut launch_cache = cache.write();
        launch_cache.clear();
        launch_cache.append(&mut launches);
    }
}

fn get_new_launches() -> Result<LaunchContainer> {
    let mut params = HashMap::new();
    params.insert("limit", "100");
    params.insert(
        "fields",
        "vidURLs,status,name,rocket,lsp,net,location,tbddate,tbdtime,windowstart,windowend,missions,mission",
    );

    Ok(DEFAULT_CLIENT
        .get("https://ll.thespacedevs.com/2.0.0/launch/upcoming")
        .header(AUTHORIZATION, LL_KEY.as_str())
        .query(&params)
        .send()?
        .error_for_status()?
        .json()?)
}

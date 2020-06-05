use reqwest::Result;
use serde::Deserialize;
use serenity::prelude::RwLock;
use std::{collections::HashMap, sync::Arc, thread::sleep, time::Duration};

use crate::{
    models::launches::{LaunchData, LaunchInfo},
    utils::constants::DEFAULT_CLIENT,
};

#[derive(Deserialize)]
struct LaunchContainer {
    pub launches: Vec<LaunchInfo>,
}

pub fn launch_tracking(cache: Arc<RwLock<Vec<LaunchData>>>) {
    loop {
        println!("getting launch information");

        let mut launches: Vec<LaunchData> = match get_new_launches() {
            Ok(ls) => ls
                .launches
                .into_iter()
                .map(|l| LaunchData::from(l))
                .collect(),
            Err(e) => {
                dbg!(e);
                sleep(Duration::from_secs(60));
                continue;
            },
        };
        launches.sort_by_key(|l| l.net);

        println!("got {} launches", launches.len());

        {
            let mut launch_cache = cache.write();
            launch_cache.clear();
            launch_cache.append(&mut launches);
        }

        sleep(Duration::from_secs(60))
    }
}

fn get_new_launches() -> Result<LaunchContainer> {
    let mut params = HashMap::new();
    params.insert("next", "100");
    params.insert(
        "fields",
        "vidURLs,status,name,rocket,lsp,net,location,tbddate,tbdtime,windowstart,windowend,missions,mission",
    );

    Ok(DEFAULT_CLIENT
        .get("https://launchlibrary.net/1.4.1/launch")
        .query(&params)
        .send()?
        .error_for_status()?
        .json()?)
}

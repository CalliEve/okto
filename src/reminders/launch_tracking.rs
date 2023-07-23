use std::{
    collections::HashMap,
    convert::TryFrom,
    sync::Arc,
};

use chrono::Duration;
use futures::stream::{
    FuturesUnordered,
    StreamExt,
};
use mongodb::Database;
use reqwest::{
    header::AUTHORIZATION,
    Result,
};
use serenity::{
    http::Http,
    prelude::RwLock,
};

use super::{
    notify_outcome,
    notify_scrub,
};
use crate::{
    models::launches::{
        LaunchContainer,
        LaunchData,
        LaunchStatus,
    },
    utils::{
        constants::{
            DEFAULT_CLIENT,
            LL_KEY,
        },
        error_log,
    },
};

pub async fn launch_tracking(http: Arc<Http>, db: Database, cache: Arc<RwLock<Vec<LaunchData>>>) {
    println!("getting launch information");

    // Get new set of launches
    let mut launches: Vec<LaunchData> = match get_new_launches().await {
        Ok(ls) => {
            ls.results
                .into_iter()
                .map(LaunchData::from)
                .collect()
        },
        Err(e) => {
            dbg!(e);
            return;
        },
    };
    launches.sort_by_key(|l| l.net);

    // Give each launch a number
    for (i, launch) in launches
        .iter_mut()
        .enumerate()
    {
        launch.id = if let Ok(id) = i32::try_from(i) {
            id
        } else {
            return;
        };
    }

    println!("got {} launches", launches.len());

    let mut launch_cache = cache
        .write()
        .await;

    // Update launch cache and free the lock
    let old_launches = launch_cache.clone();
    *launch_cache = launches.clone();
    std::mem::drop(launch_cache);
    let five_minutes = Duration::minutes(5);

    // Send out notifications
    let notif_http = http.clone();
    let notif_res = tokio::spawn(async move {
        launches
            .iter()
            .filter_map(|nl| {
                old_launches
                    .iter()
                    .find(|ol| nl.ll_id == ol.ll_id)
                    .and_then(|ol| {
                        if nl.net > (ol.net + five_minutes) {
                            Some((ol.clone(), nl.clone()))
                        } else {
                            None
                        }
                    })
            })
            .map(|l| notify_scrub(notif_http.clone(), db.clone(), l.0, l.1))
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        launches
            .iter()
            .filter(|nl| {
                matches!(
                    nl.status,
                    LaunchStatus::Success | LaunchStatus::Failure | LaunchStatus::PartialFailure
                )
            })
            .filter(|nl| {
                old_launches
                    .iter()
                    .find(|ol| nl.ll_id == ol.ll_id)
                    .map_or(false, |ol| {
                        matches!(
                            ol.status,
                            LaunchStatus::Go
                                | LaunchStatus::Tbd
                                | LaunchStatus::InFlight
                                | LaunchStatus::Hold
                        )
                    })
            })
            .cloned()
            .map(|l| notify_outcome(notif_http.clone(), db.clone(), l))
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;
    });
    if let Err(p) = notif_res.await {
        error_log(
            http,
            format!("Panic in sending notifications: {p}"),
        )
        .await;
    }
}

async fn get_new_launches() -> Result<LaunchContainer> {
    let mut params = HashMap::new();
    params.insert("limit", "100");
    params.insert("mode", "detailed");

    DEFAULT_CLIENT
        .get("https://ll.thespacedevs.com/2.0.0/launch/upcoming/")
        .header(AUTHORIZATION, LL_KEY.as_str())
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

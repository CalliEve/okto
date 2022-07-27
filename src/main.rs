#![recursion_limit = "128"]
#![warn(clippy::pedantic)]
#![allow(clippy::unreadable_literal)] // because of discord IDs being long numbers 
#![allow(clippy::module_name_repetitions)] // makes some struct names clearer
#![allow(clippy::let_underscore_drop)] // looks better when not caring if it errored imo
#![allow(clippy::semicolon_if_nothing_returned)] // looks better imo
#![allow(clippy::cast_possible_wrap)] // for mongo bson some u64 ids need to be cast to i64
#![allow(clippy::wildcard_imports)] // the commands/events structure of serenity requires these
#![allow(clippy::used_underscore_binding)] // the commands/events structure of serenity requires these
#![allow(clippy::too_many_lines)] // TODO: refactor some functions to be smaller
#![allow(clippy::non_ascii_literal)] // I want to use emojis uwu

mod commands;
mod event_handling;
mod events;
mod launch_tracking;
mod models;
mod reminder_tracking;
mod utils;

use std::{
    collections::HashMap,
    env,
    sync::Arc,
};

use commands::{
    general::*,
    help::*,
    launches::*,
    pictures::*,
    reminders::*,
};
use event_handling::Handler;
use launch_tracking::launch_tracking;
use models::caches::{
    DatabaseKey,
    EmbedSessionsKey,
    InteractionKey,
    LaunchesCacheKey,
    PictureCacheKey,
};
use mongodb::Client as MongoClient;
use reminder_tracking::reminder_tracking;
use serenity::{
    client::Client,
    framework::standard::StandardFramework,
    model::gateway::GatewayIntents,
    prelude::{
        RwLock,
        TypeMap,
    },
};
use utils::{
    error_log,
    preloading::preload_data,
};

use crate::models::caches::CommandListKey;

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("no bot token");
    let application_id: u64 = env::var("DISCORD_ID")
        .expect("no application id")
        .parse()
        .expect("provided application id is not an integer");

    let mongo_uri = if let Ok(user) = env::var("MONGO_USER") {
        format!(
            "mongodb://{}:{}@{}:27017",
            user,
            &env::var("MONGO_PASSWORD").expect("mongo password"),
            &env::var("MONGO_HOST").unwrap_or_else(|_| "mongodb".to_owned())
        )
    } else {
        "mongodb://mongo:27017".to_owned()
    };

    let framework = StandardFramework::new().after(|ctx, msg, cmd_name, error| {
        Box::pin(async move {
            //  Print out an error if it happened
            if let Err(why) = error {
                let _ = msg
                    .channel_id
                    .send_message(&ctx.http, |m| {
                        m.content("Oh no, an error happened.\nPlease try again at a later time")
                    })
                    .await;
                error_log(
                    &ctx.http,
                    format!("An error happened in {}:\n```{:?}```", cmd_name, why),
                )
                .await
            }
        })
    });

    let slash_framework = okto_framework::create_framework!(
        &token,
        application_id,
        // the commands:
        help,
        ping,
        invite,
        info,
        websites,
        peopleinspace,
        iss,
        exoplanet,
        earthpic,
        spacepic,
        spirit,
        opportunity,
        curiosity,
        perseverance,
        nextlaunch,
        listlaunches,
        launchinfo,
        filtersinfo,
        notifychannel,
        notifyme
    );

    let data_map = {
        println!("Preparing caches");
        let mut data = TypeMap::new();
        data.insert::<CommandListKey>(slash_framework.get_command_list());
        data.insert::<EmbedSessionsKey>(HashMap::new());
        data.insert::<InteractionKey>(models::caches::InteractionHandlerHolder(Vec::new()));
        data.insert::<PictureCacheKey>(preload_data().await);
        data.insert::<LaunchesCacheKey>(Arc::new(RwLock::new(Vec::new())));
        data.insert::<DatabaseKey>(
            MongoClient::with_uri_str(&mongo_uri)
                .await
                .unwrap()
                .database("okto"),
        );
        data
    };

    // create the intents for the gateway
    let mut intents = GatewayIntents::empty();
    intents.insert(GatewayIntents::GUILDS);
    intents.insert(GatewayIntents::DIRECT_MESSAGES);

    let mut client = Client::builder(&token, intents)
        .application_id(application_id)
        .framework(framework)
        .type_map(data_map)
        .event_handler(Handler::new(slash_framework))
        .await
        .expect("Error creating client");

    if let Some(launches_cache) = client
        .data
        .read()
        .await
        .get::<LaunchesCacheKey>()
    {
        if let Some(db) = client
            .data
            .read()
            .await
            .get::<DatabaseKey>()
        {
            let launches_cache_clone = launches_cache.clone();
            let http_clone = client
                .cache_and_http
                .http
                .clone();
            let db_clone = db.clone();
            tokio::spawn(reminder_tracking(
                http_clone,
                launches_cache_clone,
                db_clone,
            ));
        }
    }

    println!("Starting the bot");
    if let Err(why) = client
        .start()
        .await
    {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#![recursion_limit = "128"]
#![warn(clippy::pedantic)]
#![allow(clippy::unreadable_literal)] // because of discord IDs being long numbers
#![allow(clippy::module_name_repetitions)] // makes some struct names clearer
#![allow(clippy::semicolon_if_nothing_returned)] // looks better imo
#![allow(clippy::cast_possible_wrap)] // for mongo bson some u64 ids need to be cast to i64
#![allow(clippy::wildcard_imports)] // the commands/events structure of serenity requires these
#![allow(clippy::used_underscore_binding)] // the commands/events structure of serenity requires these
#![allow(clippy::too_many_lines)] // TODO: refactor some functions to be smaller
#![allow(clippy::non_ascii_literal)] // I want to use emojis uwu

mod commands;
mod events;
mod models;
mod reminders;
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
use models::caches::{
    CommandListKey,
    DatabaseKey,
    EmbedSessionsKey,
    InteractionKey,
    LaunchesCacheKey,
    PictureCacheKey,
};
use mongodb::Client as MongoClient;
use serenity::{
    all::ApplicationId,
    builder::CreateMessage,
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

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("no bot token");
    let application_id: ApplicationId = env::var("DISCORD_ID")
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
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                            .content("Oh no, an error happened.\nPlease try again at a later time"),
                    )
                    .await;
                error_log(
                    &ctx.http,
                    format!("An error happened in {cmd_name}:\n```{why}```",),
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
    intents.insert(GatewayIntents::GUILD_MESSAGES);

    let mut client = Client::builder(&token, intents)
        .application_id(application_id)
        .framework(framework)
        .type_map(data_map)
        .event_handler(events::Handler::new(slash_framework))
        .await
        .expect("Error creating client");

    let (http_clone, launches_cache_clone, db_clone) = if let Some(launches_cache) = client
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
                .http
                .clone();
            let db_clone = db.clone();
            (
                http_clone,
                launches_cache_clone,
                db_clone,
            )
        } else {
            panic!("No database key or connection")
        }
    } else {
        panic!("No launches cache key")
    };
    tokio::spawn(reminders::reminder_tracking(
        http_clone,
        launches_cache_clone,
        db_clone,
    ));

    println!("Starting the bot");
    if let Err(why) = client
        .start()
        .await
    {
        eprintln!("An error occurred while running the client: {why}",);
    }
}

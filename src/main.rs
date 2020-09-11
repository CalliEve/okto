mod commands;
mod event_handling;
mod events;
mod launch_tracking;
mod models;
mod reminder_tracking;
mod utils;

use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};

use mongodb::{
    bson::{self, doc},
    sync::Client as MongoClient,
};
use serenity::{
    client::{Client, Context},
    framework::standard::{
        help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions,
        StandardFramework,
    },
    model::prelude::{ChannelId, Message, UserId},
    prelude::RwLock,
};

use commands::{general::*, launches::*, pictures::*, reminders::*, settings::*};
use launch_tracking::launch_tracking;
use models::{
    caches::{DatabaseKey, EmbedSessionsKey, LaunchesCacheKey, PictureCacheKey, WaitForKey},
    settings::GuildSettings,
};
use reminder_tracking::reminder_tracking;
use utils::preloading::preload_data;

#[help]
fn help_cmd(
    context: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

fn main() {
    // Login with a bot token from the environment
    let mut client = Client::new(
        &env::var("DISCORD_TOKEN").expect("no bot token"),
        event_handling::Handler,
    )
    .expect("Error creating client");

    let mongo_uri = if let Ok(user) = env::var("MONGO_USER") {
        format!(
            "mongodb://{}:{}@{}:27017",
            user,
            &env::var("MONGO_PASSWORD").expect("mongo password"),
            &env::var("MONGO_HOST").unwrap_or("mongodb".to_owned())
        )
    } else {
        "mongodb://mongo:27017".to_owned()
    };

    {
        let mut data = client.data.write();
        data.insert::<EmbedSessionsKey>(HashMap::new());
        data.insert::<WaitForKey>(HashMap::new());
        data.insert::<PictureCacheKey>(preload_data());
        data.insert::<LaunchesCacheKey>(Arc::new(RwLock::new(Vec::new())));
        data.insert::<DatabaseKey>(
            MongoClient::with_uri_str(&mongo_uri)
                .unwrap()
                .database("okto"),
        );
    }

    if let Some(launches_cache) = client.data.read().get::<LaunchesCacheKey>() {
        if let Some(db) = client.data.read().get::<DatabaseKey>() {
            let launches_cache_clone = launches_cache.clone();
            let http_clone = client.cache_and_http.http.clone();
            let db_clone = db.clone();
            std::thread::spawn(|| reminder_tracking(http_clone, launches_cache_clone, db_clone));
        }
    }

    client.with_framework(
        StandardFramework::new()
            .configure(|c| {
                c.prefix(";")
                    .owners(vec![247745860979392512.into()].into_iter().collect())
                    .dynamic_prefix(|ctx: &mut Context, msg: &Message| {
                        if msg.guild_id.is_none() {
                            println!("Message not in guild: {}", &msg.content);
                            return None;
                        }

                        let db = if let Some(db) = ctx.data.read().get::<DatabaseKey>() {
                            db.clone()
                        } else {
                            return None;
                        };

                        let res = db
                            .collection("general_settings")
                            .find_one(doc! { "guild": msg.guild_id.unwrap().0 }, None);

                        if res.is_err() {
                            println!("Error in getting prefix: {:?}", res.unwrap_err());
                            return None;
                        }

                        res.unwrap()
                            .and_then(|c| {
                                let settings = bson::from_bson::<GuildSettings>(c.into());
                                if settings.is_err() {
                                    println!(
                                        "Error in getting prefix: {:?}",
                                        settings.unwrap_err()
                                    );
                                    return None;
                                }
                                return Some(settings.unwrap());
                            })
                            .map(|s| s.prefix)
                    })
            })
            .group(&GENERAL_GROUP)
            .group(&PICTURES_GROUP)
            .group(&LAUNCHES_GROUP)
            .group(&REMINDERS_GROUP)
            .group(&SETTINGS_GROUP)
            .help(&HELP_CMD)
            .after(|ctx, msg, cmd_name, error| {
                //  Print out an error if it happened
                if let Err(why) = error {
                    println!("Error in {}: {:?}", cmd_name, &why);
                    let _ = msg.channel_id.send_message(&ctx.http, |m| {
                        m.content("Oh no, an error happened.\nPlease try again at a later time")
                    });
                    let _ = ChannelId(447876053109702668).send_message(&ctx.http, |m| {
                        m.content(format!(
                            "An error happened in {}:\n```{:?}```",
                            cmd_name, why
                        ))
                    });
                }
            }),
    );

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}

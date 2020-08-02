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

use mongodb::sync::Client as MongoClient;
use serenity::{
    client::{Client, Context},
    framework::standard::{
        help_commands, macros::help, Args, CommandGroup, CommandResult, HelpOptions,
        StandardFramework,
    },
    model::prelude::{Message, UserId},
    prelude::RwLock,
};

use commands::{general::*, launches::*, pictures::*, reminders::*};
use models::caches::{
    DatabaseKey, EmbedSessionsKey, LaunchesCacheKey, PictureCacheKey, WaitForKey,
};
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
        &env::var("DISCORD_TOKEN").expect("token"),
        event_handling::Handler,
    )
    .expect("Error creating client");

    {
        let mut data = client.data.write();
        data.insert::<EmbedSessionsKey>(HashMap::new());
        data.insert::<WaitForKey>(HashMap::new());
        data.insert::<PictureCacheKey>(preload_data());
        data.insert::<LaunchesCacheKey>(Arc::new(RwLock::new(Vec::new())));
        data.insert::<DatabaseKey>(
            MongoClient::with_uri_str("mongodb://mongo:27017")
                .unwrap()
                .database("okto"),
        );
    }

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("!;")) // set the bot's prefix to "~"
            .group(&GENERAL_GROUP)
            .group(&PICTURES_GROUP)
            .group(&LAUNCHES_GROUP)
            .group(&REMINDERS_GROUP)
            .help(&HELP_CMD)
            .after(|ctx, msg, cmd_name, error| {
                //  Print out an error if it happened
                if let Err(why) = error {
                    println!("Error in {}: {:?}", cmd_name, why);
                    let _ = msg.channel_id
                        .send_message(&ctx.http, |m| {
                            m.content("Oh no, an error happened.\nPlease try again at a later time")
                        });
                }
            }),
    );

    // start listening for events by starting a single shard
    if let Err(why) = client.start() {
        println!("An error occurred while running the client: {:?}", why);
    }
}

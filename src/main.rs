mod commands;
mod events;
mod launch_tracking;
mod models;
mod utils;

use serenity::{
    framework::StandardFramework,
    prelude::{Client, RwLock},
};
use std::{env, sync::Arc};

use commands::{general::*, launches::*, pictures::*};
use models::caches::{LaunchesCacheKey, PictureCacheKey};
use utils::preloading::preload_data;

fn main() {
    // Login with a bot token from the environment
    let mut client = Client::new(&env::var("DISCORD_TOKEN").expect("token"), events::Handler)
        .expect("Error creating client");

    {
        let mut data = client.data.write();
        data.insert::<PictureCacheKey>(preload_data());
        data.insert::<LaunchesCacheKey>(Arc::new(RwLock::new(Vec::new())));
    }

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("!;")) // set the bot's prefix to "~"
            .group(&GENERAL_GROUP)
            .group(&PICTURES_GROUP)
            .group(&LAUNCHES_GROUP)
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

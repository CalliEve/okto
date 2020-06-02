use serenity::{
    model::id::GuildId,
    prelude::{Context, EventHandler},
};

use crate::{launch_tracking::launch_tracking, models::caches::LaunchesCacheKey};

pub struct Handler;

impl EventHandler for Handler {
    fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        {
            let cache = ctx.cache.read();
            println!(
                "############\n\
                Logged in as:\n{}\n{}\n\
                guilds: {}\n\
                Users: {}\n\
                ############",
                cache.user.name,
                cache.user.id,
                cache.all_guilds().len(),
                cache.users.len()
            )
        }

        if let Some(launches_cache) = ctx.data.read().get::<LaunchesCacheKey>() {
            let launches_cache_clone = launches_cache.clone();
            std::thread::spawn(|| launch_tracking(launches_cache_clone));
        }
    }
}

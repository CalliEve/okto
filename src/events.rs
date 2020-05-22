use serenity::{
    model::id::GuildId,
    prelude::{Context, EventHandler},
};

pub struct Handler;

impl EventHandler for Handler {
    fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
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
}

use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage},
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{
        channel::{Message, ReactionType},
        id::EmojiId,
    },
    prelude::{Context, RwLock},
};

#[group]
#[commands(notifychannel)]
struct Reminders;

#[command]
fn notifychannel(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    Ok(())
}

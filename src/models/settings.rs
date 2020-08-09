use serde::{Deserialize, Serialize};
use serenity::model::id::GuildId;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GuildSettings {
    pub guild: GuildId,
    pub prefix: String,
}

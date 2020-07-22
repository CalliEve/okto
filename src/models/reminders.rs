use serde::{Deserialize, Serialize};
use serenity::model::id::{ChannelId, GuildId, RoleId, UserId};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Reminder {
    pub minutes: i64,
    pub channels: Vec<ChannelReminder>,
    pub users: Vec<UserReminder>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChannelReminder {
    pub id: u64,
    pub server: GuildId,
    pub channel: ChannelId,
    pub filters: Vec<String>,
    pub mentions: Vec<RoleId>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserReminder {
    pub id: u64,
    pub user: UserId,
    pub filters: Vec<String>,
}

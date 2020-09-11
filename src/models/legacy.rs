use super::settings::GuildSettings;

use serde::{Deserialize, Serialize};
use serenity::model::id::GuildId;
use std::str::FromStr;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Legacy {
    pub settings: Vec<LegacySetting>,
    pub users: Vec<LegacyUser>,
    pub channels: Vec<LegacyChannel>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LegacySetting {
    pub id: String,
    pub prefix: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LegacyUser {
    pub id: String,
    #[serde(rename = "24h")]
    #[serde(default)]
    pub t_24h: Option<String>,
    #[serde(rename = "12h")]
    #[serde(default)]
    pub t_12h: Option<String>,
    #[serde(rename = "6h")]
    #[serde(default)]
    pub t_6h: Option<String>,
    #[serde(rename = "3h")]
    #[serde(default)]
    pub t_3h: Option<String>,
    #[serde(rename = "1h")]
    #[serde(default)]
    pub t_1h: Option<String>,
    #[serde(rename = "30m")]
    #[serde(default)]
    pub t_30m: Option<String>,
    #[serde(rename = "15m")]
    #[serde(default)]
    pub t_15m: Option<String>,
    #[serde(rename = "5m")]
    #[serde(default)]
    pub t_5m: Option<String>,
    #[serde(rename = "1m")]
    #[serde(default)]
    pub t_1m: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LegacyChannel {
    pub id: String,
    #[serde(rename = "24h")]
    #[serde(default)]
    pub t_24h: Option<String>,
    #[serde(rename = "12h")]
    #[serde(default)]
    pub t_12h: Option<String>,
    #[serde(rename = "6h")]
    #[serde(default)]
    pub t_6h: Option<String>,
    #[serde(rename = "3h")]
    #[serde(default)]
    pub t_3h: Option<String>,
    #[serde(rename = "1h")]
    #[serde(default)]
    pub t_1h: Option<String>,
    #[serde(rename = "30m")]
    #[serde(default)]
    pub t_30m: Option<String>,
    #[serde(rename = "15m")]
    #[serde(default)]
    pub t_15m: Option<String>,
    #[serde(rename = "5m")]
    #[serde(default)]
    pub t_5m: Option<String>,
    #[serde(rename = "1m")]
    #[serde(default)]
    pub t_1m: Option<String>,
}

impl From<LegacySetting> for GuildSettings {
    fn from(s: LegacySetting) -> Self {
        Self {
            guild: GuildId::from(u64::from_str(&s.id).unwrap()),
            prefix: s.prefix,
        }
    }
}

impl LegacyUser {
    pub fn to_vec<'a>(&'a self) -> Vec<&'a str> {
        let mut res: Vec<&'a str> = Vec::new();

        if self.t_24h.is_some() {
            res.push("24h");
        }
        if self.t_12h.is_some() {
            res.push("12h");
        }
        if self.t_6h.is_some() {
            res.push("6h");
        }
        if self.t_3h.is_some() {
            res.push("3h");
        }
        if self.t_1h.is_some() {
            res.push("1h");
        }
        if self.t_30m.is_some() {
            res.push("30m");
        }
        if self.t_15m.is_some() {
            res.push("15m");
        }
        if self.t_5m.is_some() {
            res.push("5m");
        }
        if self.t_1m.is_some() {
            res.push("1m");
        }

        res
    }
}

impl LegacyChannel {
    pub fn to_vec<'a>(&'a self) -> Vec<&'a str> {
        let mut res: Vec<&'a str> = Vec::new();

        if self.t_24h.is_some() {
            res.push("24h");
        }
        if self.t_12h.is_some() {
            res.push("12h");
        }
        if self.t_6h.is_some() {
            res.push("6h");
        }
        if self.t_3h.is_some() {
            res.push("3h");
        }
        if self.t_1h.is_some() {
            res.push("1h");
        }
        if self.t_30m.is_some() {
            res.push("30m");
        }
        if self.t_15m.is_some() {
            res.push("15m");
        }
        if self.t_5m.is_some() {
            res.push("5m");
        }
        if self.t_1m.is_some() {
            res.push("1m");
        }

        res
    }
}

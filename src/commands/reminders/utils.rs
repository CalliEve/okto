use std::{
    fmt::{
        self,
        Display,
    },
    sync::Arc,
};

use mongodb::Database;
use regex::Regex;
use serenity::{
    model::id::{
        ChannelId,
        GuildId,
        UserId,
    },
    prelude::RwLock,
};

use crate::{
    events::statefulembed::EmbedSession,
    models::caches::DatabaseKey,
    utils::constants::{
        WORD_FILTER_REGEX,
        WORD_REGEX,
    },
};

#[derive(Copy, Clone)]
pub enum ID {
    Channel((ChannelId, GuildId)),
    User(UserId),
}

impl ID {
    pub fn guild_specific(&self) -> bool {
        matches!(self, Self::Channel(_))
    }
}

pub async fn get_db(ses: &Arc<RwLock<EmbedSession>>) -> Option<Database> {
    if let Some(db) = ses
        .read()
        .await
        .data
        .read()
        .await
        .get::<DatabaseKey>()
    {
        Some(db.clone())
    } else {
        eprintln!("Could not get a database");
        None
    }
}

#[derive(Copy, Clone)]
pub enum State {
    On,
    Off,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::On => write!(f, "ON"),
            Self::Off => write!(f, "OFF"),
        }
    }
}

impl AsRef<bool> for State {
    fn as_ref(&self) -> &bool {
        match self {
            Self::On => &true,
            Self::Off => &false,
        }
    }
}

pub fn filter_from_string_input(input: String) -> String {
    if WORD_REGEX.is_match(&input) {
        format!(r"(?i)\b{input}\b")
    } else {
        input
    }
}

pub fn regex_filter_to_string(regex: &Regex) -> String {
    let filter = regex
        .as_str()
        .to_owned();
    if WORD_FILTER_REGEX.is_match(&filter) {
        filter
            .trim_end_matches(r"\b")
            .to_owned()
            .trim_start_matches(r"(?i)\b")
            .to_owned()
    } else {
        filter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_to_string() {
        let regex1 = Regex::new(r"(?i)\bstarlink\b").unwrap();
        let regex2 = Regex::new(r"(?i)\bfalcon (heavy|9)\b").unwrap();

        assert_eq!(
            regex_filter_to_string(&regex1),
            "starlink".to_owned()
        );
        assert_eq!(
            regex_filter_to_string(&regex2),
            r"(?i)\bfalcon (heavy|9)\b"
        )
    }
}

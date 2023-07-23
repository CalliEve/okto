use std::{
    fmt::{
        self,
        Display,
    },
    io::ErrorKind as IoErrorKind,
    sync::Arc,
};

use mongodb::{
    bson::{
        self,
        doc,
    },
    error::{
        Error as MongoError,
        ErrorKind as MongoErrorKind,
        Result as MongoResult,
    },
    Database,
};
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
    models::{
        caches::DatabaseKey,
        reminders::{
            GuildSettings,
            UserSettings,
        },
    },
    utils::constants::{
        WORD_FILTER_REGEX,
        WORD_REGEX,
    },
};

pub async fn get_user_settings(db: &Database, id: u64) -> MongoResult<UserSettings> {
    db.collection("user_settings")
        .find_one(doc! { "user": id as i64 }, None)
        .await?
        .ok_or_else(|| {
            MongoError::from(MongoErrorKind::Io(Arc::new(
                IoErrorKind::NotFound.into(),
            )))
        })
        .and_then(|d| bson::from_document::<UserSettings>(d).map_err(Into::into))
}

pub async fn get_guild_settings(db: &Database, id: u64) -> MongoResult<GuildSettings> {
    db.collection("guild_settings")
        .find_one(doc! { "guild": id as i64 }, None)
        .await?
        .ok_or_else(|| {
            MongoError::from(MongoErrorKind::Io(Arc::new(
                IoErrorKind::NotFound.into(),
            )))
        })
        .and_then(|d| bson::from_document::<GuildSettings>(d).map_err(Into::into))
}

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

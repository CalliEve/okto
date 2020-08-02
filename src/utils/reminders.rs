use std::io::ErrorKind as IoErrorKind;

use mongodb::{
    bson::{self, doc},
    error::{Error as MongoError, ErrorKind as MongoErrorKind, Result as MongoResult},
    sync::Database,
};

use crate::models::reminders::{GuildSettings, UserSettings};

pub fn get_user_settings(db: &Database, id: u64) -> MongoResult<UserSettings> {
    Ok(bson::from_bson(
        db.collection("user_settings")
            .find_one(doc! { "user": id }, None)?
            .ok_or(MongoError::from(MongoErrorKind::Io(
                IoErrorKind::NotFound.into(),
            )))?
            .into(),
    )?)
}

pub fn get_guild_settings(db: &Database, id: u64) -> MongoResult<GuildSettings> {
    Ok(bson::from_bson(
        db.collection("guild_settings")
            .find_one(doc! { "guild": id }, None)?
            .ok_or(MongoError::from(MongoErrorKind::Io(
                IoErrorKind::NotFound.into(),
            )))?
            .into(),
    )?)
}

use std::io::ErrorKind as IoErrorKind;

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

use crate::models::reminders::{
    GuildSettings,
    UserSettings,
};

pub async fn get_user_settings(db: &Database, id: u64) -> MongoResult<UserSettings> {
    db.collection("user_settings")
        .find_one(doc! { "user": id }, None)
        .await?
        .ok_or_else(|| MongoError::from(MongoErrorKind::Io(IoErrorKind::NotFound.into())))
        .and_then(|d| bson::from_document::<UserSettings>(d).map_err(|e| e.into()))
}

pub async fn get_guild_settings(db: &Database, id: u64) -> MongoResult<GuildSettings> {
    db.collection("guild_settings")
        .find_one(doc! { "guild": id }, None)
        .await?
        .ok_or_else(|| MongoError::from(MongoErrorKind::Io(IoErrorKind::NotFound.into())))
        .and_then(|d| bson::from_document::<GuildSettings>(d).map_err(|e| e.into()))
}

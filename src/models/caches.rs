use std::{
    collections::HashMap,
    sync::Arc,
};

use mongodb::Database;
use okto_framework::structs::Command;
use serenity::{
    model::id::MessageId,
    prelude::{
        RwLock,
        TypeMapKey,
    },
};

use super::{
    launches::LaunchData,
    pictures::MarsRoverPicture,
};
use crate::events::{
    interaction_handler::InteractionHandler,
    statefulembed::EmbedSession,
};

#[derive(Debug, Clone)]
pub struct PictureDataCache {
    pub curiosity_mardi: Vec<MarsRoverPicture>,
    pub exoplanets: Vec<String>,
    pub host_stars: Vec<String>,
}

pub struct PictureCacheKey;

impl TypeMapKey for PictureCacheKey {
    type Value = PictureDataCache;
}

pub struct LaunchesCacheKey;

impl TypeMapKey for LaunchesCacheKey {
    type Value = Arc<RwLock<Vec<LaunchData>>>;
}

pub struct EmbedSessionsKey;

impl TypeMapKey for EmbedSessionsKey {
    type Value = HashMap<MessageId, Arc<RwLock<EmbedSession>>>;
}

pub struct InteractionKey;

impl TypeMapKey for InteractionKey {
    type Value = InteractionHandlerHolder;
}

pub struct InteractionHandlerHolder(pub Vec<InteractionHandler>);

pub struct DatabaseKey;

impl TypeMapKey for DatabaseKey {
    type Value = Database;
}

pub struct CommandListKey;

impl TypeMapKey for CommandListKey {
    type Value = Vec<&'static Command>;
}

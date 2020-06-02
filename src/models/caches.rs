use std::sync::Arc;

use serenity::prelude::{RwLock, TypeMapKey};

use super::{launches::LaunchData, pictures::MarsRoverPicture};

#[derive(Debug, Clone)]
pub struct PictureDataCache {
    pub hubble_pics: Vec<i32>,
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

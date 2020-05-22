use serenity::prelude::TypeMapKey;

use super::pictures::MarsRoverPicture;

#[derive(Debug, Clone)]
pub struct PictureDataCache {
    pub hubble_pics: Vec<i32>,
    pub curiosity_mardi: Vec<MarsRoverPicture>,
    pub exoplanets: Vec<String>,
    pub host_stars: Vec<String>,
}

pub struct PictureCacheContainerKey;

impl TypeMapKey for PictureCacheContainerKey {
    type Value = PictureDataCache;
}

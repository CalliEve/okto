use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct EPICImage {
    pub image: String,
    pub date: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct APODImage {
    pub explanation: Option<String>,
    pub title: String,
    pub url: String,
    pub date: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MarsRoverCamera {
    pub id: u8,
    pub name: String,
    pub rover_id: u8,
    pub full_name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MarsRoverPicture {
    pub id: i32,
    pub img_src: String,
    pub earth_date: String,
    pub camera: MarsRoverCamera,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MarsRoverPictureRes {
    pub photos: Vec<MarsRoverPicture>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HubbleImage {
    pub width: u32,
    pub file_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HubbleImageSource {
    pub description: Option<String>,
    pub image_files: Vec<HubbleImage>,
    pub name: String,
}

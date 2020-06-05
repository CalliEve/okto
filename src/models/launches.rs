use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::utils::serde::{datetime_formatting, duration, string_option};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LaunchData {
    pub id: i32,
    pub launch_name: String,
    pub status: LaunchStatus,
    pub payload: String,
    pub vid_urls: Vec<String>,
    pub vehicle: String,
    pub location: String,
    pub rocket_img: Option<String>,
    #[serde(with = "datetime_formatting")]
    pub net: NaiveDateTime,
    #[serde(with = "duration")]
    pub launch_window: Duration,
    pub mission_type: Option<MissionType>,
    pub mission_description: Option<String>,
    pub lsp: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LaunchInfo {
    pub id: i32,
    pub name: String,
    pub status: LaunchStatus,
    #[serde(with = "datetime_formatting")]
    pub net: NaiveDateTime,
    #[serde(with = "datetime_formatting")]
    pub windowstart: NaiveDateTime,
    #[serde(with = "datetime_formatting")]
    pub windowend: NaiveDateTime,
    pub location: LocationInfo,
    pub rocket: RocketInfo,
    pub missions: Vec<MissionInfo>,
    pub lsp: AgencyInfo,
    #[serde(rename = "vidURLs")]
    pub vid_urls: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RocketInfo {
    pub id: i32,
    pub name: String,
    pub configuration: String,
    pub familyname: String,
    #[serde(rename = "wikiURL")]
    #[serde(with = "string_option")]
    pub wiki_url: Option<String>,
    #[serde(rename = "infoURLs")]
    pub info_urls: Option<Vec<String>>,
    #[serde(rename = "imageURL")]
    #[serde(with = "string_option")]
    pub image_url: Option<String>,
    pub agencies: Option<Vec<AgencyInfo>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AgencyInfo {
    pub id: i32,
    pub name: String,
    pub abbrev: String,
    #[serde(rename = "countryCode")]
    pub country_code: String,
    #[serde(rename = "wikiURL")]
    #[serde(with = "string_option")]
    pub wiki_url: Option<String>,
    #[serde(rename = "type")]
    pub agency_type: AgencyType,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LocationInfo {
    pub id: i32,
    pub pads: Vec<PadInfo>,
    pub name: String,
    #[serde(rename = "countryCode")]
    pub country_code: String,
    #[serde(rename = "wikiURL")]
    #[serde(with = "string_option")]
    pub wiki_url: Option<String>,
    #[serde(rename = "infoURL")]
    #[serde(with = "string_option")]
    pub info_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PadInfo {
    pub id: i32,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(rename = "mapURL")]
    #[serde(with = "string_option")]
    pub map_url: Option<String>,
    #[serde(rename = "wikiURL")]
    #[serde(with = "string_option")]
    pub wiki_url: Option<String>,
    #[serde(rename = "infoURL")]
    #[serde(with = "string_option")]
    pub info_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MissionInfo {
    pub id: i32,
    pub name: String,
    pub description: String,
    #[serde(rename = "wikiURL")]
    #[serde(with = "string_option")]
    pub wiki_url: Option<String>,
    #[serde(rename = "typeName")]
    pub type_name: String,
    #[serde(rename = "type")]
    pub mission_type: MissionType,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LaunchStatus {
    Go = 1,
    TBD = 2,
    Success = 3,
    Failure = 4,
    Hold = 5,
    InFlight = 6,
    PartialFailure = 7,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum AgencyType {
    Government = 1,
    Multinational = 2,
    Commercial = 3,
    Educational = 4,
    Private = 5,
    Unknown = 6,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum MissionType {
    EarthScience = 1,
    PlanetaryScience = 2,
    Astrophysics = 3,
    Heliophysics = 4,
    HumanExploration = 5,
    RoboticExploration = 6,
    Government = 7,
    Tourism = 8,
    Unknown = 9,
    Communications = 10,
    Resupply = 11,
    Suborbital = 12,
    TestFlight = 13,
    DedicatedRideshare = 14,
    Navigation = 15,
    MicrogravityResearch = 16,
}

impl LaunchStatus {
    pub fn to_emoji(&self) -> &str {
        match self {
            LaunchStatus::Go => "<:certain:447805610482728964>",
            LaunchStatus::TBD => "<:uncertain:447805624923717642>",
            _ => "<:offline:484650962368069633>",
        }
    }
}

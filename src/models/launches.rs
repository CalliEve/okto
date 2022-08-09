use chrono::{
    Duration,
    NaiveDateTime,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_repr::{
    Deserialize_repr,
    Serialize_repr,
};

use crate::utils::serde::{
    datetime_formatting,
    duration,
    string_option,
};

#[derive(Deserialize)]
pub struct LaunchContainer {
    pub results: Vec<LaunchInfo>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LaunchData {
    pub id: i32,
    pub ll_id: String,
    pub launch_name: String,
    pub status: LaunchStatus,
    pub payload: String,
    pub vid_urls: Vec<VidURL>,
    pub vehicle: String,
    pub location: String,
    pub rocket_img: Option<String>,
    #[serde(with = "datetime_formatting")]
    pub net: NaiveDateTime,
    #[serde(with = "duration")]
    pub launch_window: Duration,
    pub mission_type: String,
    pub mission_description: String,
    pub lsp: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LaunchInfo {
    pub id: String,
    pub name: String,
    pub status: StatusMap,
    #[serde(with = "datetime_formatting")]
    pub net: NaiveDateTime,
    #[serde(with = "datetime_formatting")]
    pub window_start: NaiveDateTime,
    #[serde(with = "datetime_formatting")]
    pub window_end: NaiveDateTime,
    pub pad: PadInfo,
    pub rocket: RocketInfo,
    pub mission: Option<MissionInfo>,
    pub launch_service_provider: Option<AgencyInfo>,
    #[serde(rename = "vidURLs")]
    pub vid_urls: Option<Vec<VidURL>>,
    #[serde(with = "string_option")]
    pub image: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RocketInfo {
    pub id: i32,
    pub configuration: RocketConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RocketConfig {
    pub id: i32,
    pub name: String,
    pub family: String,
    pub full_name: String,
    pub variant: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AgencyInfo {
    pub id: i32,
    pub name: String,
    #[serde(with = "string_option")]
    pub url: Option<String>,
    #[serde(rename = "type")]
    #[serde(with = "string_option")]
    pub agency_type: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LocationInfo {
    pub id: i32,
    pub pads: Option<Vec<PadInfo>>,
    pub name: String,
    pub country_code: String,
    pub total_launch_count: i32,
    pub total_landing_count: i32,
    pub map_image: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PadInfo {
    pub id: i32,
    pub name: String,
    pub latitude: Option<String>,
    pub longitude: Option<String>,
    pub agency_id: Option<i32>,
    pub location: LocationInfo,
    #[serde(with = "string_option")]
    pub map_url: Option<String>,
    #[serde(with = "string_option")]
    pub info_url: Option<String>,
    pub total_launch_count: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MissionInfo {
    pub id: i32,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub mission_type: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LauncherDetail {
    pub id: i32,
    #[serde(with = "string_option")]
    pub serial_number: Option<String>,
    pub status: String,
    pub details: String,
    pub flight_proven: bool,
    #[serde(with = "string_option")]
    pub image_url: Option<String>,
    pub successful_landings: i32,
    pub attempted_landings: i32,
    pub flights: String,
    pub last_launch_date: String,
    pub first_launch_date: String,
    pub launcher_config: LauncherConfigDetail,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LauncherConfigDetail {
    pub id: i32,
    pub name: String,
    pub family: String,
    pub full_name: String,
    #[serde(with = "string_option")]
    pub image_url: Option<String>,
    pub description: String,
    pub variant: String,
    pub length: i32,
    pub max_stage: Option<i32>,
    pub min_stage: Option<i32>,
    pub diameter: i32,
    #[serde(with = "string_option")]
    pub info_url: Option<String>,
    #[serde(with = "string_option")]
    pub wiki_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VidURL {
    pub priority: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: String,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum LaunchStatus {
    Go = 1,
    Tbd = 2,
    Success = 3,
    Failure = 4,
    Hold = 5,
    InFlight = 6,
    PartialFailure = 7,
}

impl LaunchStatus {
    pub fn as_str(&self) -> &str {
        match self {
            LaunchStatus::Go => "Go",
            LaunchStatus::Tbd => "TBD",
            LaunchStatus::Failure => "Failure",
            LaunchStatus::Success => "Success",
            LaunchStatus::InFlight => "In Flight",
            LaunchStatus::Hold => "Hold",
            LaunchStatus::PartialFailure => "Partial Failure",
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StatusMap {
    pub id: LaunchStatus,
    pub name: String,
}

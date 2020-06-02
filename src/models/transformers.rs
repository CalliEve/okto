use super::launches::{LaunchData, LaunchInfo};
use chrono::Duration;

impl From<LaunchInfo> for LaunchData {
    fn from(info: LaunchInfo) -> LaunchData {
        LaunchData {
            id: info.id,
            launch_name: info.name,
            status: info.status,
            payload: info
                .missions
                .first()
                .map(|m| m.name.as_str())
                .unwrap_or("Payload Unknown")
                .to_owned(),
            vid_urls: info.vid_urls.unwrap_or_default(),
            vehicle: info.rocket.name,
            location: info
                .location
                .pads
                .first()
                .map(|p| p.name.as_str())
                .unwrap_or(info.location.name.as_str())
                .to_owned(),
            net: info.net,
            launch_window: info.windowend - info.windowstart,
            rocket_img: info.rocket.image_url,
            mission_type: info.missions.first().map(|m| m.mission_type.clone()),
            mission_description: info.missions.first().map(|m| m.description.clone()),
            lsp: info.lsp.name,
        }
    }
}

use std::str::FromStr;

use super::launches::{
    LaunchData,
    LaunchInfo,
};

impl From<LaunchInfo> for LaunchData {
    fn from(mut info: LaunchInfo) -> LaunchData {
        if let Some(urls) = info
            .vid_urls
            .as_mut()
        {
            urls.sort_by_key(|u| u.priority);
            urls.dedup_by_key(|u| {
                if let Ok(link) = url::Url::from_str(&u.url) {
                    if let Some(domain) = link.domain() {
                        return domain.to_owned();
                    };
                };
                u.title
                    .clone()
                    .unwrap_or_else(|| "Title Unknown".to_owned())
            });
        };

        LaunchData {
            id: 0,
            ll_id: info.id,
            launch_name: info.name,
            status: info
                .status
                .id,
            payload: info
                .mission
                .clone()
                .map_or_else(
                    || String::from("payload unknown"),
                    |m| m.name,
                ),
            vid_urls: info
                .vid_urls
                .unwrap_or_default(),
            vehicle: info
                .rocket
                .configuration
                .full_name,
            location: info
                .pad
                .name,
            net: info.net,
            launch_window: info.window_end - info.window_start,
            rocket_img: info.image,
            mission_type: info
                .mission
                .clone()
                .map_or_else(
                    || String::from("mission type unknown"),
                    |m| m.mission_type,
                ),
            mission_description: info
                .mission
                .clone()
                .map_or_else(
                    || String::from("mission description unknown"),
                    |m| m.description,
                ),
            lsp: info
                .launch_service_provider
                .map_or_else(
                    || String::from("Unknown launch provider"),
                    |l| l.name,
                ),
        }
    }
}

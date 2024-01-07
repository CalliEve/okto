use crate::{
    models::{
        launches::LaunchData,
        reminders::ReminderSettings,
    },
    utils::constants::LAUNCH_AGENCIES,
};

pub(super) fn passes_filters<T>(settings: &T, l: &LaunchData) -> bool
where
    T: ReminderSettings,
{
    let passes_agencies = !settings
        .get_filters()
        .iter()
        .filter_map(|filter| LAUNCH_AGENCIES.get(filter.as_str()))
        .any(|agency| *agency == l.lsp);

    let passes_agency_allows = settings
        .get_allow_filters()
        .is_empty()
        || settings
            .get_allow_filters()
            .iter()
            .filter_map(|filter| LAUNCH_AGENCIES.get(filter.as_str()))
            .any(|agency| *agency == l.lsp);

    let passes_payloads = !settings
        .get_payload_filters()
        .iter()
        .any(|re| re.is_match(&l.payload));

    passes_agencies && passes_agency_allows && passes_payloads
}

#[cfg(test)]
mod tests {
    use chrono::{
        Duration,
        NaiveDateTime,
    };
    use regex::Regex;
    use serenity::model::id::{
        ChannelId,
        GuildId,
    };

    use super::*;
    use crate::models::{
        launches::LaunchStatus,
        reminders::GuildSettings,
    };

    fn create_fake_launches() -> Vec<LaunchData> {
        vec![
            LaunchData {
                id: 1,
                ll_id: String::new(),
                launch_name: "Falcon 9 Block 5".into(),
                status: LaunchStatus::Go,
                payload: "Starlink Group 6-2".into(),
                vid_urls: vec![],
                vehicle: "Falcon 9 Block 5".into(),
                location: "Launch Complex 39A".into(),
                rocket_img: None,
                net: NaiveDateTime::from_timestamp_opt(1635409251, 0).unwrap(),
                launch_window: Duration::seconds(60),
                mission_type: String::new(),
                mission_description: String::new(),
                lsp: "SpaceX".into(),
            },
            LaunchData {
                id: 2,
                ll_id: String::new(),
                launch_name: "Atlas V 551".into(),
                status: LaunchStatus::Go,
                payload: "STP-3".into(),
                vid_urls: vec![],
                vehicle: "Atlas V 551".into(),
                location: "Space Launch Complex 41".into(),
                rocket_img: None,
                net: NaiveDateTime::from_timestamp_opt(1635409251, 0).unwrap(),
                launch_window: Duration::seconds(60),
                mission_type: String::new(),
                mission_description: String::new(),
                lsp: "United Launch Alliance".into(),
            },
        ]
    }

    #[test]
    fn no_filters() {
        let launches = create_fake_launches();
        let settings = GuildSettings {
            guild: GuildId::new(429307670730637312),
            filters: vec![],
            allow_filters: vec![],
            payload_filters: vec![],
            mentions: vec![],
            scrub_notifications: true,
            outcome_notifications: true,
            mention_others: true,
            notifications_channel: Some(ChannelId::new(429307774804033536)),
        };

        assert!(passes_filters(&settings, &launches[0]));
        assert!(passes_filters(&settings, &launches[1]));
    }

    #[test]
    fn block_filters() {
        let launches = create_fake_launches();
        let settings = GuildSettings {
            guild: GuildId::new(429307670730637312),
            filters: vec!["ula".into()],
            allow_filters: vec![],
            payload_filters: vec![],
            mentions: vec![],
            scrub_notifications: true,
            outcome_notifications: true,
            mention_others: true,
            notifications_channel: Some(ChannelId::new(429307774804033536)),
        };

        assert!(passes_filters(&settings, &launches[0]));
        assert!(!passes_filters(&settings, &launches[1]));
    }

    #[test]
    fn allow_filters() {
        let launches = create_fake_launches();
        let settings = GuildSettings {
            guild: GuildId::new(429307670730637312),
            filters: vec![],
            allow_filters: vec!["ula".into()],
            payload_filters: vec![],
            mentions: vec![],
            scrub_notifications: true,
            outcome_notifications: true,
            mention_others: true,
            notifications_channel: Some(ChannelId::new(429307774804033536)),
        };

        assert!(!passes_filters(&settings, &launches[0]));
        assert!(passes_filters(&settings, &launches[1]));
    }

    #[test]
    fn payload_filters() {
        let launches = create_fake_launches();
        let settings = GuildSettings {
            guild: GuildId::new(429307670730637312),
            filters: vec![],
            allow_filters: vec![],
            payload_filters: vec![Regex::new(r"(?im)\bstarlink\b").unwrap()],
            mentions: vec![],
            scrub_notifications: true,
            outcome_notifications: true,
            mention_others: true,
            notifications_channel: Some(ChannelId::new(429307774804033536)),
        };

        assert!(!passes_filters(&settings, &launches[0]));
        assert!(passes_filters(&settings, &launches[1]));
    }
}

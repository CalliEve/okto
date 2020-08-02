use chrono::{Duration, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor},
    http::Http,
    model::id::ChannelId,
    utils::Colour,
};

use super::constants::{DEFAULT_COLOR, DEFAULT_ICON, ID_REGEX, MENTION_REGEX};

lazy_static! {
    static ref WEEK_REGEX: Regex = Regex::new(r"(^|\b)([0-9]+)[wW]").unwrap();
    static ref DAY_REGEX: Regex = Regex::new(r"(^|\b)([0-9]+)[dD]").unwrap();
    static ref HOUR_REGEX: Regex = Regex::new(r"(^|\b)([0-9]+)[hH]").unwrap();
    static ref MINUTE_REGEX: Regex = Regex::new(r"(^|\b)([0-9]+)[mM]").unwrap();
}

pub fn cutoff_on_last_dot(text: &str, length: usize) -> &str {
    let mut last: usize = 0;
    for (index, character) in text.chars().enumerate() {
        if character == '.' {
            last = index
        } else if index >= length - 1 {
            if last != 0 {
                return &text[..(last + 1)];
            } else {
                return &text[..length];
            }
        }
    }
    text
}

pub fn default_embed<'a>(
    embed: &'a mut CreateEmbed,
    content: &str,
    success: bool,
) -> &'a mut CreateEmbed {
    embed
        .author(|a: &mut CreateEmbedAuthor| a.name("OKTO").icon_url(DEFAULT_ICON))
        .color(if success {
            DEFAULT_COLOR.into()
        } else {
            Colour::RED
        })
        .description(content)
        .timestamp(&Utc::now())
}

pub fn format_duration(dur: Duration, include_seconds: bool) -> String {
    let days = dur.num_days();
    let hours = dur.num_hours() - days * 24;
    let minutes = dur.num_minutes() - dur.num_hours() * 60;
    let seconds = dur.num_seconds() - dur.num_minutes() * 60;

    let mut res = String::new();

    match days {
        1 => res.push_str(&format!("{} day", days)),
        x if x > 1 => res.push_str(&format!("{} days", days)),
        _ => {}
    }

    if hours > 0 {
        if days > 0 && minutes == 0 && seconds == 0 {
            res.push_str(" and ");
        } else if days > 0 {
            res.push_str(", ")
        }

        if hours == 1 {
            res.push_str(&format!("{} hour", hours));
        } else {
            res.push_str(&format!("{} hours", hours));
        }
    }

    if minutes > 0 {
        if (days > 0 || hours > 0) && seconds == 0 {
            res.push_str(" and ");
        } else if days > 0 || hours > 0 {
            res.push_str(", ")
        }

        if minutes == 1 {
            res.push_str(&format!("{} minute", minutes));
        } else {
            res.push_str(&format!("{} minutes", minutes));
        }
    }

    if seconds > 0 && include_seconds {
        if days > 0 || hours > 0 || minutes > 0 {
            res.push_str(" and ");
        }

        if seconds == 1 {
            res.push_str(&format!("{} second", seconds));
        } else {
            res.push_str(&format!("{} seconds", seconds));
        }
    }

    res
}

pub fn parse_duration(text: &str) -> Duration {
    let mut dur = Duration::zero();

    if let Some(num_raw) = WEEK_REGEX.captures(text) {
        if let Ok(num) = num_raw.get(2).unwrap().as_str().parse() {
            dur = dur + Duration::weeks(num)
        }
    }

    if let Some(num_raw) = DAY_REGEX.captures(text) {
        if let Ok(num) = num_raw.get(2).unwrap().as_str().parse() {
            dur = dur + Duration::days(num)
        }
    }

    if let Some(num_raw) = HOUR_REGEX.captures(text) {
        if let Ok(num) = num_raw.get(2).unwrap().as_str().parse() {
            dur = dur + Duration::hours(num)
        }
    }

    if let Some(num_raw) = MINUTE_REGEX.captures(text) {
        if let Ok(num) = num_raw.get(2).unwrap().as_str().parse() {
            dur = dur + Duration::minutes(num)
        }
    }

    dur
}

pub fn parse_id(text: &str) -> Option<u64> {
    if ID_REGEX.is_match(text) {
        return text.parse().ok();
    }

    if let Some(captures) = MENTION_REGEX.captures(text) {
        return captures.get(1).unwrap().as_str().parse().ok();
    }

    None
}

pub fn temp_message(channel: ChannelId, http: impl AsRef<Http>, text: &str, delay: Duration) {
    if let Ok(message) = channel.send_message(&http, |m| m.content(text)) {
        std::thread::sleep(delay.to_std().unwrap());
        let _ = channel.delete_message(http, message.id);
    }
}

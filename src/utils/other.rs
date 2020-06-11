use chrono::{Duration, Utc};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor},
    utils::Colour,
};

use super::constants::{DEFAULT_COLOR, DEFAULT_ICON};

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
        .color(
            if success {
                DEFAULT_COLOR.into()
            } else {
                Colour::RED
            },
        )
        .description(content)
        .timestamp(&Utc::now())
}

pub fn format_duration(dur: Duration) -> String {
    let days = dur.num_days();
    let hours = dur.num_hours() - days * 24;
    let minutes = dur.num_minutes() - dur.num_hours() * 60;
    let seconds = dur.num_seconds() - dur.num_minutes() * 60;

    let mut res = String::new();

    match days {
        1 => res.push_str(&format!("{} day", days)),
        x if x > 1 => res.push_str(&format!("{} days", days)),
        _ => {},
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

    if seconds > 0 {
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

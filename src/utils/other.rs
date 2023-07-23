use std::{fmt::Write, ops::Add};

use lazy_static::lazy_static;
use regex::Regex;
use chrono::{
    Duration,
    Utc,
};
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
    },
    http::Http,
    model::{
        application::component::ButtonStyle,
        channel::ReactionType,
        id::ChannelId,
    },
    utils::Colour,
};

use super::constants::{
    CHECK_EMOJI,
    DEFAULT_COLOR,
    DEFAULT_ICON,
    EXIT_EMOJI,
    FINAL_PAGE_EMOJI,
    FIRST_PAGE_EMOJI,
    LAST_PAGE_EMOJI,
    NEXT_PAGE_EMOJI,
    PROGRADE,
    RETROGRADE,
};
use crate::events::statefulembed::ButtonType;

pub fn cutoff_on_last_dot(text: &str, length: usize) -> &str {
    let mut last: usize = 0;
    for (index, character) in text
        .chars()
        .enumerate()
    {
        if character == '.' {
            last = index
        } else if index >= length - 1 {
            if last == 0 {
                return &text[..length];
            }
            return &text[..=last];
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
        .author(|a: &mut CreateEmbedAuthor| {
            a.name("OKTO")
                .icon_url(DEFAULT_ICON)
        })
        .color(
            if success {
                DEFAULT_COLOR.into()
            } else {
                Colour::RED
            },
        )
        .description(content)
        .timestamp(Utc::now())
}

pub fn format_duration(dur: Duration, include_seconds: bool) -> String {
    let days = dur.num_days();
    let hours = dur.num_hours() - days * 24;
    let minutes = dur.num_minutes() - dur.num_hours() * 60;
    let seconds = dur.num_seconds() - dur.num_minutes() * 60;

    let mut res = String::new();

    match days {
        1 => write!(res, "{} day", days).expect("write to String: can't fail"),
        x if x > 1 => write!(res, "{} days", days).expect("write to String: can't fail"),
        _ => {},
    }

    if hours > 0 {
        if days > 0 && minutes == 0 && seconds == 0 {
            res.push_str(" and ");
        } else if days > 0 {
            res.push_str(", ")
        }

        if hours == 1 {
            write!(res, "{} hour", hours).expect("write to String: can't fail");
        } else {
            write!(res, "{} hours", hours).expect("write to String: can't fail");
        }
    }

    if minutes > 0 {
        if (days > 0 || hours > 0) && seconds == 0 {
            res.push_str(" and ");
        } else if days > 0 || hours > 0 {
            res.push_str(", ")
        }

        if minutes == 1 {
            write!(res, "{} minute", minutes).expect("write to String: can't fail");
        } else {
            write!(res, "{} minutes", minutes).expect("write to String: can't fail");
        }
    }

    if seconds > 0 && include_seconds {
        if days > 0 || hours > 0 || minutes > 0 {
            res.push_str(" and ");
        }

        if seconds == 1 {
            write!(res, "{} second", seconds).expect("write to String: can't fail");
        } else {
            write!(res, "{} seconds", seconds).expect("write to String: can't fail");
        }
    }
    if res.is_empty() {
        res = "unknown".to_owned();
    }

    res
}

lazy_static! {
    pub static ref DAYS_REGEX: Regex = Regex::new("([0-9]+) days?").unwrap();
    pub static ref HOURS_REGEX: Regex = Regex::new("([0-9]+) hours?").unwrap();
    pub static ref MINUTES_REGEX: Regex = Regex::new("([0-9]+) minutes?").unwrap();
    pub static ref SECONDS_REGEX: Regex = Regex::new("([0-9]+) seconds?").unwrap();
}

pub fn parse_duration(input: &str) -> Duration {
    let mut dur = Duration::zero();

    if let Some(captures) = DAYS_REGEX.captures(&input) {
        if let Some(days_str) = captures.get(1) {
            if let Ok(days) = days_str.as_str().parse::<i64>() {
                dur = dur.add(Duration::days(days));
            }
        }
    }

    if let Some(captures) = HOURS_REGEX.captures(&input) {
        if let Some(hour_str) = captures.get(1) {
            if let Ok(hours) = hour_str.as_str().parse::<i64>() {
                dur = dur.add(Duration::hours(hours));
            }
        }
    }

    if let Some(captures) = MINUTES_REGEX.captures(&input) {
        if let Some(minutes_str) = captures.get(1) {
            if let Ok(minutes) = minutes_str.as_str().parse::<i64>() {
                dur = dur.add(Duration::minutes(minutes));
            }
        }
    }

    if let Some(captures) = SECONDS_REGEX.captures(&input) {
        if let Some(seconds_str) = captures.get(1) {
            if let Ok(seconds) = seconds_str.as_str().parse::<i64>() {
                dur = dur.add(Duration::seconds(seconds));
            }
        }
    }

    dur
}

#[allow(dead_code)]
const DEBUG_CHANNEL: ChannelId = ChannelId(771669392399532063);
const ERROR_CHANNEL: ChannelId = ChannelId(447876053109702668);

#[allow(dead_code)]
pub async fn debug_log(http: impl AsRef<Http>, text: &str) {
    let _ = DEBUG_CHANNEL
        .send_message(&http, |m| m.content(text))
        .await;
}

pub async fn error_log(http: impl AsRef<Http>, text: impl AsRef<str>) {
    eprintln!("{}", text.as_ref());
    let _ = ERROR_CHANNEL
        .send_message(&http, |m| {
            m.embed(|em| {
                em.description(text.as_ref())
                    .color(Colour::RED)
                    .timestamp(Utc::now())
            })
        })
        .await;
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum StandardButton {
    First,
    Last,
    Forward,
    Back,
    Exit,
    Submit,
    Prograde,
    Retrograde,
}

impl StandardButton {
    pub fn to_button(self) -> ButtonType {
        match self {
            Self::Last => {
                ButtonType {
                    label: "Last page".to_owned(),
                    style: ButtonStyle::Secondary,
                    emoji: Some(ReactionType::from(FINAL_PAGE_EMOJI)),
                }
            },
            Self::First => {
                ButtonType {
                    label: "First page".to_owned(),
                    style: ButtonStyle::Secondary,
                    emoji: Some(ReactionType::from(FIRST_PAGE_EMOJI)),
                }
            },
            Self::Forward => {
                ButtonType {
                    label: "Forward one page".to_owned(),
                    style: ButtonStyle::Secondary,
                    emoji: Some(ReactionType::from(NEXT_PAGE_EMOJI)),
                }
            },
            Self::Back => {
                ButtonType {
                    label: "Back one page".to_owned(),
                    style: ButtonStyle::Secondary,
                    emoji: Some(ReactionType::from(LAST_PAGE_EMOJI)),
                }
            },
            Self::Exit => {
                ButtonType {
                    label: "Exit".to_owned(),
                    style: ButtonStyle::Danger,
                    emoji: Some(ReactionType::from(EXIT_EMOJI)),
                }
            },
            Self::Submit => {
                ButtonType {
                    label: "Submit".to_owned(),
                    style: ButtonStyle::Success,
                    emoji: Some(ReactionType::from(CHECK_EMOJI)),
                }
            },
            Self::Prograde => {
                ButtonType {
                    label: "Next".to_owned(),
                    style: ButtonStyle::Secondary,
                    emoji: Some(PROGRADE.clone()),
                }
            },
            Self::Retrograde => {
                ButtonType {
                    label: "Back".to_owned(),
                    style: ButtonStyle::Secondary,
                    emoji: Some(RETROGRADE.clone()),
                }
            },
        }
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => {
            f.to_uppercase()
                .collect::<String>()
                + c.as_str()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_to_str() {
        let dur = Duration::days(2).add(Duration::hours(8)).add(Duration::minutes(23)).add(Duration::seconds(1));

        assert_eq!(format_duration(dur, true), "2 days, 8 hours, 23 minutes and 1 second");
    }

    #[test]
    fn str_to_duration() {
        let dur = Duration::days(2).add(Duration::hours(8)).add(Duration::minutes(23)).add(Duration::seconds(1));

        assert_eq!(parse_duration("2 days, 8 hours, 23 minutes and 1 second"), dur);
    }
}

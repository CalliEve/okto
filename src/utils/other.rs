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
        channel::ReactionType,
        id::ChannelId,
        interactions::message_component::ButtonStyle,
    },
    utils::Colour,
};

use super::constants::{
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
    if res.is_empty() {
        res = "unknown".to_owned();
    }

    res
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
    Prograde,
    Retrograde,
}

impl StandardButton {
    pub fn to_button(self) -> ButtonType {
        match self {
            Self::Last => ButtonType {
                label: "Last page".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(ReactionType::from(FINAL_PAGE_EMOJI)),
            },
            Self::First => ButtonType {
                label: "First page".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(ReactionType::from(FIRST_PAGE_EMOJI)),
            },
            Self::Forward => ButtonType {
                label: "Forward one page".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(ReactionType::from(NEXT_PAGE_EMOJI)),
            },
            Self::Back => ButtonType {
                label: "Back one page".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(ReactionType::from(LAST_PAGE_EMOJI)),
            },
            Self::Exit => ButtonType {
                label: "Exit".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(EXIT_EMOJI)),
            },
            Self::Prograde => ButtonType {
                label: "Next".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(PROGRADE.clone()),
            },
            Self::Retrograde => ButtonType {
                label: "Back".to_owned(),
                style: ButtonStyle::Secondary,
                emoji: Some(RETROGRADE.clone()),
            },
        }
    }
}

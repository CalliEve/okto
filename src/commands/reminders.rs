use std::{io::ErrorKind as IoErrorKind, sync::Arc};

use chrono::{Duration, Utc};
use mongodb::{
    bson::{self, doc},
    error::{Error as MongoError, ErrorKind as MongoErrorKind, Result as MongoResult},
    options::UpdateOptions,
    sync::Database,
};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor},
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{
        channel::Message,
        id::{ChannelId, GuildId, RoleId, UserId},
    },
    prelude::{Context, RwLock},
};

use crate::{
    events::{
        statefulembed::{EmbedSession, StatefulEmbed},
        waitfor::{WaitFor, WaitPayload},
    },
    models::{caches::DatabaseKey, reminders::Reminder},
    utils::{
        constants::*,
        format_duration, parse_duration, parse_id,
        reminders::{get_guild_settings, get_user_settings},
        temp_message,
    },
};

#[group]
#[commands(notifychannel, notifyme)]
struct Reminders;

#[command]
#[only_in(guild)]
fn notifychannel(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    if msg.guild_id.is_none() {
        return Ok(());
    }

    let target_channel = if let Some(channel_id) = args
        .current()
        .map(|c| parse_id(c))
        .flatten()
        .map(|c| ChannelId(c))
    {
        if let Some(channel) = channel_id.to_channel_cached(&ctx) {
            channel.id()
        } else {
            msg.channel_id
        }
    } else {
        msg.channel_id
    };

    let ses = EmbedSession::new_show(&ctx, msg.channel_id, msg.author.id)?;

    main_menu(ses, ID::Channel((target_channel, msg.guild_id.unwrap())));

    Ok(())
}

#[command]
fn notifyme(ctx: &mut Context, msg: &Message) -> CommandResult {
    let dm = if msg.guild_id.is_some() {
        msg.author.create_dm_channel(&ctx)?.id
    } else {
        msg.channel_id
    };

    let ses = EmbedSession::new_show(&ctx, dm, msg.author.id)?;

    main_menu(ses, ID::User(msg.author.id));

    Ok(())
}

// ---- pages functions ----

fn main_menu(ses: Arc<RwLock<EmbedSession>>, id: ID) {
    let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
        e.color(DEFAULT_COLOR)
            .timestamp(&Utc::now())
            .author(|a: &mut CreateEmbedAuthor| {
                a.name("Launch Reminder Settings").icon_url(DEFAULT_ICON)
            })
    });

    let reminder_ses = ses.clone();
    em.add_field(
        "Reminders",
        "Set at which times you want to get launch reminders",
        false,
        &"⏰".into(),
        move || reminders_page(reminder_ses.clone(), id),
    );

    let filters_ses = ses.clone();
    em.add_field(
        "Filters",
        "Set which agencies to filter out of launch reminders",
        false,
        &"📝".into(),
        move || filters_page(filters_ses.clone(), id),
    );

    if id.guild_specific() {
        let mention_ses = ses.clone();
        em.add_field(
            "Mentions",
            "Set which roles should be mentioned when posting reminders",
            false,
            &"🔔".into(),
            move || mentions_page(mention_ses.clone(), id),
        );
    }

    let close_ses = ses.clone();
    em.add_field(
        "Close",
        "Close this menu",
        false,
        &"❌".into(),
        move || {
            close_ses
                .clone()
                .read()
                .message
                .as_ref()
                .map(|m| m.delete(&close_ses.clone().read().http));
        },
    );

    let res = em.show();
    if res.is_err() {
        dbg!(res.unwrap_err());
    }
}

fn reminders_page(ses: Arc<RwLock<EmbedSession>>, id: ID) {
    let reminders_res = get_reminders(&ses, id);
    let description = match reminders_res {
        Ok(reminders) if !reminders.is_empty() => {
            let mut text = "The following reminders have been set:".to_owned();
            for reminder in &reminders {
                text.push_str(&format!(
                    "\n- {}",
                    format_duration(reminder.get_duration(), false)
                ))
            }
            text
        }
        _ => "No reminders have been set yet".to_owned(),
    };

    let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
        e.color(DEFAULT_COLOR)
            .timestamp(&Utc::now())
            .author(|a: &mut CreateEmbedAuthor| a.name("Launch Reminders").icon_url(DEFAULT_ICON))
            .description(description)
    });

    let add_ses = ses.clone();
    em.add_field(
        "Add Reminder",
        "Add a new reminder",
        false,
        &PROGRADE,
        move || {
            let inner_ses = add_ses.clone();
            let channel_id = inner_ses.read().channel;
            let user_id = inner_ses.read().author;
            let wait_ses = add_ses.clone();

            WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                if let WaitPayload::Message(message) = payload {
                    let dur = parse_duration(&message.content);
                    if !dur.is_zero() {
                        add_reminder(&wait_ses.clone(), id, dur);
                    }
                    reminders_page(wait_ses.clone(), id)
                }
            })
            .send_explanation(
                "Send how long before the launch you want the reminder.\nPlease give the time in the format of `1w 2d 3h 4m`",
                &inner_ses.read().http,
            )
            .listen(inner_ses.read().data.clone());
        },
    );

    let remove_ses = ses.clone();
    em.add_field(
        "Remove Reminder",
        "Remove a reminder",
        false,
        &RETROGRADE,
        move || {
            let inner_ses = remove_ses.clone();
            let channel_id = inner_ses.read().channel;
            let user_id = inner_ses.read().author;
            let wait_ses = remove_ses.clone();

            WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                if let WaitPayload::Message(message) = payload {
                    remove_reminder(&wait_ses.clone(), id, parse_duration(&message.content));
                    reminders_page(wait_ses.clone(), id)
                }
            })
            .send_explanation(
                "Send the time of the reminder you want to remove.\nPlease give the time in the format of `1w 2d 3h 4m`",
                &inner_ses.read().http,
            )
            .listen(inner_ses.read().data.clone());
        },
    );

    em.add_field(
        "Back",
        "Go back to main menu",
        false,
        &"❌".into(),
        move || main_menu(ses.clone(), id),
    );

    let res = em.show();
    if res.is_err() {
        dbg!(res.unwrap_err());
    }
}

fn filters_page(ses: Arc<RwLock<EmbedSession>>, id: ID) {
    let db = if let Some(db_res) = get_db(&ses) {
        db_res
    } else {
        return;
    };

    let description = match id {
        ID::Channel(channel_id) => {
            let settings_res = get_guild_settings(&db, channel_id.1.into());
            match settings_res {
                Ok(settings) if !settings.filters.is_empty() => {
                    let mut text = "The following agency filters have been set:".to_owned();
                    for filter in &settings.filters {
                        text.push_str(&format!("\n`{}`", filter))
                    }
                    text
                }
                _ => "No agency filters have been set yet".to_owned(),
            }
        }
        ID::User(user_id) => {
            let settings_res = get_user_settings(&db, user_id.into());
            match settings_res {
                Ok(settings) if !settings.filters.is_empty() => {
                    let mut text = "The following agency filters have been set:".to_owned();
                    for filter in &settings.filters {
                        text.push_str(&format!("\n`{}`", filter))
                    }
                    text
                }
                _ => "No agency filters have been set yet".to_owned(),
            }
        }
    };

    let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
        e.color(DEFAULT_COLOR)
            .timestamp(&Utc::now())
            .author(|a: &mut CreateEmbedAuthor| {
                a.name("Launch Agency Filters").icon_url(DEFAULT_ICON)
            })
            .description(description)
    });

    let add_ses = ses.clone();
    em.add_field(
        "Add Filter",
        "Add a new filter",
        false,
        &PROGRADE,
        move || {
            let inner_ses = add_ses.clone();
            let channel_id = inner_ses.read().channel;
            let user_id = inner_ses.read().author;
            let wait_ses = add_ses.clone();

            WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                println!("triggered add filter payload handler");
                if let WaitPayload::Message(message) = payload {
                    add_filter(&wait_ses.clone(), id, message.content);
                    filters_page(wait_ses.clone(), id)
                }
            })
            .send_explanation(
                "Send the filter name of the agency you do not want to receive reminders for",
                &inner_ses.read().http,
            )
            .listen(inner_ses.read().data.clone());
        },
    );

    let remove_ses = ses.clone();
    em.add_field(
        "Remove Filter",
        "Remove a filter",
        false,
        &RETROGRADE,
        move || {
            let inner_ses = remove_ses.clone();
            let channel_id = inner_ses.read().channel;
            let user_id = inner_ses.read().author;
            let wait_ses = remove_ses.clone();

            WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                if let WaitPayload::Message(message) = payload {
                    remove_filter(&wait_ses.clone(), id, message.content);
                    filters_page(wait_ses.clone(), id)
                }
            })
            .send_explanation(
                "Send the filter name of the agency you want to receive reminders for again",
                &inner_ses.read().http,
            )
            .listen(inner_ses.read().data.clone());
        },
    );

    em.add_field(
        "Back",
        "Go back to main menu",
        false,
        &"❌".into(),
        move || main_menu(ses.clone(), id),
    );

    let res = em.show();
    if res.is_err() {
        dbg!(res.unwrap_err());
    }
}

fn mentions_page(ses: Arc<RwLock<EmbedSession>>, id: ID) {
    let db = if let Some(db_res) = get_db(&ses) {
        db_res
    } else {
        return;
    };

    let description = match id {
        ID::Channel((_, guild_id)) => {
            let settings_res = get_guild_settings(&db, guild_id.into());
            match settings_res {
                Ok(settings) if !settings.mentions.is_empty() => {
                    let mut text = "The following roles have been set to be mentioned:".to_owned();
                    for role_id in &settings.mentions {
                        let role_opt = role_id.to_role_cached(ses.read().cache.clone());
                        if let Some(role) = role_opt {
                            text.push_str(&format!("\n`{}`", role.name))
                        } else {
                            remove_mention(&ses, id, *role_id)
                        }
                    }
                    text
                }
                _ => "No role mentions have been set yet".to_owned(),
            }
        }
        _ => return,
    };

    let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
        e.color(DEFAULT_COLOR)
            .timestamp(&Utc::now())
            .author(|a: &mut CreateEmbedAuthor| a.name("Role Mentions").icon_url(DEFAULT_ICON))
            .description(description)
    });

    let add_ses = ses.clone();
    em.add_field(
        "Add Mention",
        "Add a new role to mention",
        false,
        &PROGRADE,
        move || {
            let inner_ses = add_ses.clone();
            let channel_id = inner_ses.read().channel;
            let user_id = inner_ses.read().author;
            let wait_ses = add_ses.clone();

            WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                if let WaitPayload::Message(message) = payload {
                    if let Some(role_id) = parse_id(&message.content) {
                        add_mention(&wait_ses.clone(), id, role_id.into());
                        mentions_page(wait_ses.clone(), id);
                    } else {
                        mentions_page(wait_ses.clone(), id);
                        temp_message(
                            channel_id,
                            wait_ses.read().http.clone(),
                            "Sorry, I can't find that role, please try again later",
                            Duration::seconds(5),
                        )
                    }
                }
            })
            .send_explanation(
                "Mention the role you want to have mentioned during launch reminders",
                &inner_ses.read().http,
            )
            .listen(inner_ses.read().data.clone());
        },
    );

    let remove_ses = ses.clone();
    em.add_field(
        "Remove Mention",
        "Remove a role to mention",
        false,
        &RETROGRADE,
        move || {
            let inner_ses = remove_ses.clone();
            let channel_id = inner_ses.read().channel;
            let user_id = inner_ses.read().author;
            let wait_ses = remove_ses.clone();

            WaitFor::message(channel_id, user_id, move |payload: WaitPayload| {
                if let WaitPayload::Message(message) = payload {
                    if let Some(role_id) = parse_id(&message.content) {
                        remove_mention(&wait_ses.clone(), id, role_id.into());
                        mentions_page(wait_ses.clone(), id);
                    } else {
                        mentions_page(wait_ses.clone(), id);
                        temp_message(
                            channel_id,
                            wait_ses.read().http.clone(),
                            "Sorry, I can't find that role, please try again later",
                            Duration::seconds(5),
                        )
                    }
                }
            })
            .send_explanation(
                "Mention the role you want to have removed from being mentioned during launch reminders", 
                &inner_ses.read().http
            )
            .listen(inner_ses.read().data.clone());
        },
    );

    em.add_field(
        "Back",
        "Go back to main menu",
        false,
        &"❌".into(),
        move || main_menu(ses.clone(), id),
    );

    let res = em.show();
    if res.is_err() {
        dbg!(res.unwrap_err());
    }
}

// ---- db functions ----

fn get_reminders(ses: &Arc<RwLock<EmbedSession>>, id: ID) -> MongoResult<Vec<Reminder>> {
    let db = if let Some(db) = get_db(&ses) {
        db
    } else {
        return Err(MongoError::from(MongoErrorKind::Io(
            IoErrorKind::NotFound.into(),
        )));
    };

    match id {
        ID::User(user_id) => Ok(bson::from_bson(
            db.collection("reminders")
                .find(doc! { "users": { "$in": [user_id.0] } }, None)?
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        )?),
        ID::Channel((channel_id, guild_id)) => Ok(bson::from_bson(
            db.collection("reminders")
                .find(
                    doc! { "channels": { "$in": [{ "channel": channel_id.0, "guild": guild_id.0 }] } },
                    None,
                )?
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        )?),
    }
}

fn add_reminder(ses: &Arc<RwLock<EmbedSession>>, id: ID, duration: Duration) {
    let db = if let Some(db) = get_db(&ses) {
        db
    } else {
        return;
    };

    let res = match id {
        ID::User(user_id) => db.collection("reminders").update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$addToSet": {
                    "users": user_id.0
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        ID::Channel((channel_id, guild_id)) => db.collection("reminders").update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$addToSet": {
                    "channels": { "channel": channel_id.0, "guild": guild_id.0 }
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
    };

    if let Err(e) = res {
        dbg!(e);
    }
}

fn remove_reminder(ses: &Arc<RwLock<EmbedSession>>, id: ID, duration: Duration) {
    let db = if let Some(db) = get_db(&ses) {
        db
    } else {
        return;
    };

    let _ = match id {
        ID::User(user_id) => db.collection("reminders").update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$pull": {
                    "users": user_id.0
                }
            },
            None,
        ),
        ID::Channel((channel_id, guild_id)) => db.collection("reminders").update_one(
            doc! {"minutes": duration.num_minutes()},
            doc! {
                "$pull": {
                    "channels": { "channel": channel_id.0, "guild": guild_id.0 }
                }
            },
            None,
        ),
    };
}

fn add_filter(ses: &Arc<RwLock<EmbedSession>>, id: ID, filter: String) {
    let db = if let Some(db) = get_db(&ses) {
        db
    } else {
        return;
    };

    if !LAUNCH_AGENCIES.contains_key(filter.as_str()) {
        println!("agencies does not contain filter {}", &filter);
        return;
    }

    let res = match id {
        ID::User(user_id) => db.collection("user_settings").update_one(
            doc! {"user": user_id.0},
            doc! {
                "$addToSet": {
                    "filters": filter
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
        ID::Channel((_, guild_id)) => db.collection("guild_settings").update_one(
            doc! {"user": guild_id.0},
            doc! {
                "$addToSet": {
                    "filters": filter
                }
            },
            Some(UpdateOptions::builder().upsert(true).build()),
        ),
    };

    if let Err(e) = res {
        dbg!(e);
    }
}

fn remove_filter(ses: &Arc<RwLock<EmbedSession>>, id: ID, filter: String) {
    let db = if let Some(db) = get_db(&ses) {
        db
    } else {
        return;
    };

    let _ = match id {
        ID::User(user_id) => db.collection("user_settings").update_one(
            doc! {"user": user_id.0},
            doc! {
                "$pull": {
                    "filters": filter
                }
            },
            None,
        ),
        ID::Channel((_, guild_id)) => db.collection("guild_settings").update_one(
            doc! {"user": guild_id.0},
            doc! {
                "$pull": {
                    "filters": filter
                }
            },
            None,
        ),
    };
}

fn add_mention(ses: &Arc<RwLock<EmbedSession>>, id: ID, role: RoleId) {
    let guild_id = if let ID::Channel((_, guild_id)) = id {
        guild_id
    } else {
        return;
    };

    let db = if let Some(db) = get_db(&ses) {
        db
    } else {
        return;
    };

    let _ = db.collection("guild_settings").update_one(
        doc! {"guild": guild_id.0},
        doc! {
            "$addToSet": {
                "mentions": role.0
            }
        },
        Some(UpdateOptions::builder().upsert(true).build()),
    );
}

fn remove_mention(ses: &Arc<RwLock<EmbedSession>>, id: ID, role: RoleId) {
    let guild_id = if let ID::Channel((_, guild_id)) = id {
        guild_id
    } else {
        return;
    };

    let db = if let Some(db) = get_db(&ses) {
        db
    } else {
        return;
    };

    let _ = db.collection("guild_settings").update_one(
        doc! {"guild": guild_id.0},
        doc! {
            "$pull": {
                "mentions": role.0
            }
        },
        None,
    );
}

// ---- utils ----

#[derive(Copy, Clone)]
enum ID {
    Channel((ChannelId, GuildId)),
    User(UserId),
}

impl ID {
    fn guild_specific(&self) -> bool {
        match self {
            Self::Channel(_) => true,
            _ => false,
        }
    }
}

fn get_db(ses: &Arc<RwLock<EmbedSession>>) -> Option<Database> {
    if let Some(db) = ses.read().data.read().get::<DatabaseKey>() {
        Some(db.clone())
    } else {
        println!("Could not get a database");
        None
    }
}

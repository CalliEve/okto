use std::{
    io::ErrorKind as IoErrorKind,
    sync::Arc,
};

use chrono::Duration;
use futures::stream::StreamExt;
use mongodb::{
    bson::{
        self,
        doc,
        document::Document,
    },
    error::{
        Error as MongoError,
        ErrorKind as MongoErrorKind,
        Result as MongoResult,
    },
    options::UpdateOptions,
    Collection,
};
use serenity::{
    model::id::{
        ChannelId,
        RoleId,
    },
    prelude::RwLock,
};

use super::utils::{
    get_db,
    ID,
};
use crate::{
    events::statefulembed::EmbedSession,
    models::reminders::Reminder
};

pub async fn get_reminders(ses: &Arc<RwLock<EmbedSession>>, id: ID) -> MongoResult<Vec<Reminder>> {
    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return Err(MongoError::from(MongoErrorKind::Io(
            Arc::new(IoErrorKind::NotFound.into()),
        )));
    };

    match id {
        ID::User(user_id) => Ok(bson::from_bson(
            db.collection::<Document>("reminders")
                .find(doc! { "users": { "$in": [user_id.0 as i64] } }, None).await?
                .collect::<Vec<Result<_, _>>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        )?),
        ID::Channel((channel_id, guild_id)) => Ok(bson::from_bson(
            db.collection::<Document>("reminders")
                .find(
                    doc! { "channels": { "$in": [{ "channel": channel_id.0 as i64, "guild": guild_id.0 as i64 }] } },
                    None,
                ).await?
                .collect::<Vec<Result<_, _>>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        )?),
    }
}

pub async fn add_reminder(ses: &Arc<RwLock<EmbedSession>>, id: ID, duration: Duration) {
    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    let collection = db.collection::<Document>("reminders");

    let result =
        match id {
            ID::User(user_id) => {
                collection.update_one(
                    doc! {"minutes": duration.num_minutes()},
                    doc! {
                        "$addToSet": {
                            "users": user_id.0 as i64
                        }
                    },
                    Some(
                        UpdateOptions::builder()
                            .upsert(true)
                            .build(),
                    ),
                )
            },
            ID::Channel((channel_id, guild_id)) => collection.update_one(
                doc! {"minutes": duration.num_minutes()},
                doc! {
                    "$addToSet": {
                        "channels": { "channel": channel_id.0 as i64, "guild": guild_id.0 as i64 }
                    }
                },
                Some(
                    UpdateOptions::builder()
                        .upsert(true)
                        .build(),
                ),
            ),
        }
        .await;

    if let Err(e) = result {
        eprintln!("error while adding reminder:");
        dbg!(e);
    }
}

pub async fn remove_reminder(ses: &Arc<RwLock<EmbedSession>>, id: ID, duration: Duration) {
    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    let collection = db.collection::<Document>("reminders");

    let result =
        match id {
            ID::User(user_id) => {
                collection.update_one(
                    doc! {"minutes": duration.num_minutes()},
                    doc! {
                        "$pull": {
                            "users": user_id.0 as i64
                        }
                    },
                    None,
                )
            },
            ID::Channel((channel_id, guild_id)) => collection.update_one(
                doc! {"minutes": duration.num_minutes()},
                doc! {
                    "$pull": {
                        "channels": { "channel": channel_id.0 as i64, "guild": guild_id.0 as i64 }
                    }
                },
                None,
            ),
        }
        .await;

    if let Err(e) = result {
        eprintln!("error while removing reminder:");
        dbg!(e);
    }
}

pub async fn add_filter(
    ses: &Arc<RwLock<EmbedSession>>,
    id: ID,
    filter: String,
    filter_type: &str,
) {
    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    let collection: Collection<Document> = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => {
            collection.update_one(
                doc! {"user": user_id.0 as i64},
                doc! {
                    "$addToSet": {
                        filter_type: filter
                    }
                },
                Some(
                    UpdateOptions::builder()
                        .upsert(true)
                        .build(),
                ),
            )
        },
        ID::Channel((_, guild_id)) => {
            collection.update_one(
                doc! {"guild": guild_id.0 as i64},
                doc! {
                    "$addToSet": {
                        filter_type: filter
                    }
                },
                Some(
                    UpdateOptions::builder()
                        .upsert(true)
                        .build(),
                ),
            )
        },
    }
    .await;

    if let Err(e) = result {
        eprintln!("error while adding filter:");
        dbg!(e);
    }
}

pub async fn remove_filter(
    ses: &Arc<RwLock<EmbedSession>>,
    id: ID,
    filter: String,
    filter_type: &str,
) {
    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    let collection: Collection<Document> = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => {
            collection.update_one(
                doc! {"user": user_id.0 as i64},
                doc! {
                    "$pull": {
                        filter_type: filter
                    }
                },
                None,
            )
        },
        ID::Channel((_, guild_id)) => {
            collection.update_one(
                doc! {"guild": guild_id.0 as i64},
                doc! {
                    "$pull": {
                        filter_type: filter
                    }
                },
                None,
            )
        },
    }
    .await;

    if let Err(e) = result {
        eprintln!("error while removing filter:");
        dbg!(e);
    }
}

pub async fn toggle_setting(ses: &Arc<RwLock<EmbedSession>>, id: ID, setting: &str, val: bool) {
    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    let collection: Collection<Document> = if id.guild_specific() {
        db.collection("guild_settings")
    } else {
        db.collection("user_settings")
    };

    let result = match id {
        ID::User(user_id) => {
            collection.update_one(
                doc! {"user": user_id.0 as i64},
                doc! {
                    "$set": {
                        setting: val
                    }
                },
                Some(
                    UpdateOptions::builder()
                        .upsert(true)
                        .build(),
                ),
            )
        },
        ID::Channel((_, guild_id)) => {
            collection.update_one(
                doc! {"guild": guild_id.0 as i64},
                doc! {
                    "$set": {
                        setting: val
                    }
                },
                Some(
                    UpdateOptions::builder()
                        .upsert(true)
                        .build(),
                ),
            )
        },
    }
    .await;

    if let Err(e) = result {
        eprintln!("error while toggling setting:");
        dbg!(e);
    }
}

pub async fn set_notification_channel(ses: &Arc<RwLock<EmbedSession>>, id: ID, channel: ChannelId) {
    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    let collection = db.collection::<Document>("guild_settings");

    let result = match id {
        ID::Channel((_, guild_id)) => {
            collection.update_one(
                doc! {"guild": guild_id.0 as i64},
                doc! {
                    "$set": {
                        "notifications_channel": channel.0 as i64
                    }
                },
                Some(
                    UpdateOptions::builder()
                        .upsert(true)
                        .build(),
                ),
            )
        },
        ID::User(_) => return,
    }
    .await;

    if let Err(e) = result {
        eprintln!("error while setting notification channel:");
        dbg!(e);
    }
}

pub async fn add_mention(ses: &Arc<RwLock<EmbedSession>>, id: ID, role: RoleId) {
    let guild_id = if let ID::Channel((_, guild_id)) = id {
        guild_id
    } else {
        return;
    };

    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    let result = db
        .collection::<Document>("guild_settings")
        .update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$addToSet": {
                    "mentions": role.0 as i64
                }
            },
            Some(
                UpdateOptions::builder()
                    .upsert(true)
                    .build(),
            ),
        )
        .await;

    if let Err(e) = result {
        eprintln!("error while adding mention:");
        dbg!(e);
    }
}

pub async fn remove_mention(ses: &Arc<RwLock<EmbedSession>>, id: ID, role: RoleId) {
    let guild_id = if let ID::Channel((_, guild_id)) = id {
        guild_id
    } else {
        return;
    };

    let db = if let Some(db) = get_db(ses).await {
        db
    } else {
        return;
    };

    let result = db
        .collection::<Document>("guild_settings")
        .update_one(
            doc! {"guild": guild_id.0 as i64},
            doc! {
                "$pull": {
                    "mentions": role.0 as i64
                }
            },
            None,
        )
        .await;

    if let Err(e) = result {
        eprintln!("error while removing mention:");
        dbg!(e);
    }
}

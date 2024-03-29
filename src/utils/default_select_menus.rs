use std::sync::Arc;

use futures::future::BoxFuture;
use serenity::{
    http::Http,
    model::{
        application::Interaction,
        id::{
            ChannelId,
            GuildId,
            RoleId,
            UserId,
        },
    },
    prelude::{
        RwLock,
        TypeMap,
    },
};

use crate::events::select_menu::SelectMenu;

pub async fn role_select_menu<F>(
    http: Arc<Http>,
    user_id: UserId,
    interaction: &Interaction,
    data: Arc<RwLock<TypeMap>>,
    exclude: Option<Vec<RoleId>>,
    include: Option<Vec<RoleId>>,
    callback: F,
) where
    F: Fn(RoleId) -> BoxFuture<'static, ()> + Send + Sync + 'static,
{
    let guild = get_guild(interaction);
    let mut roles = guild
        .roles(&http)
        .await
        .expect("Can't get roles from guild");

    if let Some(include) = include {
        roles.retain(|id, _| include.contains(id));
    }

    if let Some(exclude) = exclude {
        roles.retain(|id, _| !exclude.contains(id));
    }

    SelectMenu::builder(move |(id, _)| {
        let id: RoleId = id
            .parse()
            .expect("Got invalid role id from role select");
        callback(id)
    })
    .set_description("Select a role")
    .set_custom_id(&format!("{user_id}-role-select"))
    .set_user(user_id)
    .make_ephemeral()
    .set_options(
        roles
            .into_iter()
            .take(125)
            .map(|(k, v)| {
                (
                    k.get()
                        .to_string(),
                    v.name,
                )
            })
            .collect(),
    )
    .build()
    .unwrap()
    .listen(http, interaction, data)
    .await;
}

pub async fn channel_select_menu<F>(
    http: Arc<Http>,
    user_id: UserId,
    interaction: &Interaction,
    data: Arc<RwLock<TypeMap>>,
    callback: F,
) where
    F: Fn(ChannelId) -> BoxFuture<'static, ()> + Send + Sync + 'static,
{
    let guild = get_guild(interaction);
    let channels = guild
        .channels(&http)
        .await
        .expect("Can't get channels from guild");

    SelectMenu::builder(move |(id, _)| {
        let id = ChannelId::new(
            id.parse()
                .expect("Got invalid channel id from channel select"),
        );
        callback(id)
    })
    .set_description("Select a channel")
    .set_custom_id(&format!("{user_id}-channel-select"))
    .set_user(user_id)
    .make_ephemeral()
    .set_options(
        channels
            .into_iter()
            .take(125)
            .map(|(k, v)| {
                (
                    k.get()
                        .to_string(),
                    v.name,
                )
            })
            .collect(),
    )
    .build()
    .unwrap()
    .listen(http, interaction, data)
    .await;
}

fn get_guild(interaction: &Interaction) -> GuildId {
    match interaction {
        Interaction::Component(comp) => {
            comp.guild_id
                .expect("Trying to get channels in a non-guild menu")
        },
        Interaction::Modal(modal) => {
            modal
                .guild_id
                .expect("Trying to get channels in a non-guild modal")
        },
        Interaction::Command(cmd) => {
            cmd.guild_id
                .expect("Trying to get channels in a non-guild command")
        },
        _ => panic!("Unsupported interaction"),
    }
}

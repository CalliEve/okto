use std::{fmt::Write, sync::Arc};

use itertools::Itertools;
use mongodb::bson::{doc, document::Document, from_bson};
use okto_framework::{macros::command, structs::Command};
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, EditInteractionResponse},
    framework::standard::{macros::hook, CommandError, CommandResult},
    model::{
        application::{
            component::ButtonStyle, interaction::application_command::ApplicationCommandInteraction,
        },
        prelude::{Channel, Message, MessageType, ReactionType},
        Permissions,
    },
    prelude::{Context, RwLock},
};

use crate::{
    events::statefulembed::{ButtonType, EmbedSession, StatefulEmbed},
    models::{
        caches::{CommandListKey, DatabaseKey},
        settings::GuildSettings,
    },
    utils::{
        capitalize,
        constants::{BACK_EMOJI, DEFAULT_COLOR, DEFAULT_ICON, EXIT_EMOJI, NUMBER_EMOJIS, OWNERS},
    },
};

#[command]
#[options(
    {
        option_type: String,
        name: "command",
        description: "Get information about a specific command"
    }
)]
/// Get information about the commands within the bot
async fn help(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    let ses = EmbedSession::new(ctx, interaction.clone(), false).await?;

    if let Some(command_name) = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "command")
        .and_then(|o| {
            o.value
                .clone()
        })
        .and_then(|v| {
            v.as_str()
                .map(ToOwned::to_owned)
        })
    {
        let command = if let Some(cmd) = ctx
            .data
            .read()
            .await
            .get::<CommandListKey>()
            .unwrap()
            .iter()
            .find(|c| {
                c.options
                    .name
                    == command_name
            }) {
            *cmd
        } else {
            return Ok(());
        };

        interaction
            .edit_original_interaction_response(
                &ctx.http,
                |i: &mut EditInteractionResponse| {
                    let args = command
                        .options
                        .options
                        .iter()
                        .fold(String::new(), |acc, opt| {
                            let name = if opt.required {
                                format!("<{}> ", opt.name)
                            } else {
                                format!("[{}] ", opt.name)
                            };
                            acc + &name
                        });

                    i.embed(|e: &mut CreateEmbed| {
                        e.author(|a: &mut CreateEmbedAuthor| {
                            a.name(format!(
                                "Help /{}",
                                command
                                    .options
                                    .name
                            ))
                            .icon_url(DEFAULT_ICON)
                        })
                        .color(DEFAULT_COLOR)
                        .description(format!(
                            "**Description:** {}{}",
                            command
                                .options
                                .description,
                            if args.is_empty() {
                                String::new()
                            } else {
                                format!("\n**Arguments:** {args}")
                            }
                        ))
                    })
                },
            )
            .await?;
        return Ok(());
    }

    help_menu(ses, ctx.clone(), interaction.clone()).await;

    Ok(())
}

fn help_menu(
    ses: Arc<RwLock<EmbedSession>>,
    ctx: Context,
    interaction: ApplicationCommandInteraction,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
            .author(
                |a: &mut CreateEmbedAuthor| a.name("Help Menu").icon_url(DEFAULT_ICON)
            ).description("Use the buttons to get the descriptions for the commands in that group.\nCurrently available commands:")
        });

        let grouped = ctx
            .data
            .read()
            .await
            .get::<CommandListKey>()
            .unwrap()
            .iter()
            .sorted_by_key(|c| {
                c.info
                    .file
            })
            .group_by(|c| {
                c.info
                    .file
            })
            .into_iter()
            .fold(
                Vec::<(&str, Vec<&'static Command>)>::new(),
                |mut acc, (k, g)| {
                    acc.push((
                        k,
                        g.into_iter()
                            .copied()
                            .collect(),
                    ));
                    acc
                },
            );

        for (i, (key, group)) in grouped
            .into_iter()
            .enumerate()
        {
            let group_name = key
                .split_once('.')
                .unwrap()
                .0
                .split('/')
                .last()
                .map(capitalize)
                .unwrap();
            if group_name == "Help" {
                continue;
            }

            if allowed(&ctx, &group, &interaction)
                .await
                .unwrap_or(false)
            {
                let mut cmds = String::new();

                for command in &group {
                    if allowed(&ctx, &[command], &interaction)
                        .await
                        .unwrap_or(false)
                    {
                        write!(
                            cmds,
                            "\n- **/{}**",
                            command
                                .options
                                .name
                        )
                        .expect("write to String: can't fail");
                    }
                }

                if cmds.is_empty() {
                    continue;
                }

                let details_ses = ses.clone();
                let details_ctx = ctx.clone();
                let details_interaction = interaction.clone();
                em.add_field(
                    &group_name.clone(),
                    &cmds,
                    true,
                    &ButtonType {
                        label: group_name.clone(),
                        style: ButtonStyle::Primary,
                        emoji: Some(NUMBER_EMOJIS[i].clone()),
                    },
                    move |_| {
                        let details_ses = details_ses.clone();
                        let details_ctx = details_ctx.clone();
                        let details_group = group.clone();
                        let details_interaction = details_interaction.clone();
                        let group_name = group_name.clone();
                        Box::pin(async move {
                            command_details(
                                details_ses.clone(),
                                details_ctx,
                                details_interaction,
                                details_group,
                                group_name,
                            )
                            .await
                        })
                    },
                );
            }
        }

        em.add_option(
            &ButtonType {
                label: "Exit".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(ReactionType::from(EXIT_EMOJI)),
            },
            move |_| {
                let close_ses = ses.clone();
                Box::pin(async move {
                    let lock = close_ses
                        .read()
                        .await;
                    let _ = lock
                        .interaction
                        .delete_original_interaction_response(&lock.http)
                        .await;
                })
            },
        );

        let show_res = em
            .show()
            .await;
        if let Err(e) = show_res {
            eprintln!("Error in help: {}", e);
        }
    })
}

fn command_details(
    ses: Arc<RwLock<EmbedSession>>,
    ctx: Context,
    interaction: ApplicationCommandInteraction,
    selected_group: Vec<&'static Command>,
    group_name: String,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name(format!("{} Commands", &group_name))
                        .icon_url(DEFAULT_ICON)
                })
                .description("More Detailed information about the commands in this group")
        });

        for command in &selected_group {
            if allowed(&ctx, &[command], &interaction)
                .await
                .unwrap_or(false)
            {
                let args = command
                    .options
                    .options
                    .iter()
                    .fold(String::new(), |acc, opt| {
                        let name = if opt.required {
                            format!("<{}> ", opt.name)
                        } else {
                            format!("[{}] ", opt.name)
                        };
                        acc + &name
                    });

                em.inner
                    .field(
                        format!(
                            "/{}",
                            command
                                .options
                                .name
                        ),
                        format!(
                            "**Description:** {}{}",
                            command
                                .options
                                .description,
                            if args.is_empty() {
                                String::new()
                            } else {
                                format!("\n**Arguments:** {args}")
                            }
                        ),
                        false,
                    );
            }
        }

        em.add_field(
            "Back",
            "Back to help menu",
            false,
            &ButtonType {
                label: "Back".to_owned(),
                style: ButtonStyle::Danger,
                emoji: Some(BACK_EMOJI.into()),
            },
            move |_| {
                let back_ses = ses.clone();
                let back_ctx = ctx.clone();
                let back_interaction = interaction.clone();
                Box::pin(async move { help_menu(back_ses, back_ctx, back_interaction).await })
            },
        );

        let show_res = em
            .show()
            .await;
        if let Err(e) = show_res {
            eprintln!("Error in help: {}", e);
        }
    })
}

async fn allowed(
    ctx: &Context,
    cmds: &[&'static Command],
    interaction: &ApplicationCommandInteraction,
) -> Result<bool, CommandError> {
    if OWNERS.contains(
        &interaction
            .user
            .id,
    ) {
        return Ok(true);
    }

    let channel = interaction
        .channel_id
        .to_channel(&ctx)
        .await?;

    if cmds
        .iter()
        .all(|c| {
            !c.options
                .default_permission
        })
    {
        if let Channel::Guild(channel) = &channel {
            let guild = if let Some(guild) = interaction
                .guild_id
                .unwrap()
                .to_guild_cached(ctx)
            {
                guild
            } else {
                return Ok(false);
            };

            if interaction
                .user
                .id
                == guild.owner_id
            {
                return Ok(true);
            }

            let member = if let Some(member) = &interaction.member {
                member
            } else {
                return Ok(false);
            };

            if let Ok(perms) = guild.user_permissions_in(channel, member) {
                if !(perms.contains(Permissions::ADMINISTRATOR)
                    || perms.contains(Permissions::MANAGE_GUILD))
                {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }

    Ok(true)
}

#[hook]
pub async fn calc_prefix(ctx: &Context, msg: &Message) -> String {
    if msg
        .guild_id
        .is_none()
    {
        return ";".to_owned();
    }

    let db = if let Some(db) = ctx
        .data
        .read()
        .await
        .get::<DatabaseKey>()
    {
        db.clone()
    } else {
        eprintln!("No database found");
        return ";".to_owned();
    };

    let res = db
        .collection::<Document>("general_settings")
        .find_one(
            doc! { "guild": msg.guild_id.unwrap().0 as i64 },
            None,
        )
        .await;

    if res.is_err() {
        eprintln!(
            "Error in getting prefix: {:?}",
            res.unwrap_err()
        );
        return ";".to_owned();
    }

    res.unwrap()
        .and_then(|c| {
            let settings = from_bson::<GuildSettings>(c.into());
            if settings.is_err() {
                eprintln!(
                    "Error in getting prefix: {:?}",
                    settings.unwrap_err()
                );
                return None;
            }
            let settings = settings.unwrap();
            Some(settings)
        })
        .map_or_else(|| ";".to_owned(), |s| s.prefix)
}

pub async fn slash_command_message(ctx: &Context, msg: &Message) {
    if !msg
        .mentions_me(&ctx)
        .await
        .unwrap_or(false)
        || msg.kind == MessageType::InlineReply
    {
        return;
    }

    let _ = msg
        .reply_ping(&ctx, "Hi! OKTO has moved over to using slash-commands.\nThis means that you should use / as the prefix, for example `/help`.")
        .await;
}

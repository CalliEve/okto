use std::{collections::HashSet, sync::Arc};

use mongodb::bson::doc;
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor},
    framework::standard::{
        macros::{help, hook},
        Args,
        CommandGroup,
        CommandResult,
        CommonOptions,
        HelpOptions,
        OnlyIn,
    },
    model::prelude::{Channel, Message, ReactionType, UserId},
    prelude::{Context, RwLock},
};

use crate::{
    events::statefulembed::{EmbedSession, StatefulEmbed},
    models::{caches::DatabaseKey, settings::GuildSettings},
    utils::constants::{DEFAULT_COLOR, DEFAULT_ICON, EXIT_EMOJI, NUMBER_EMOJIS},
};

#[help]
async fn help_cmd(
    ctx: &Context,
    msg: &Message,
    _args: Args,
    _help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let ses = EmbedSession::new(&ctx, msg.channel_id, msg.author.id);

    help_menu(ses, ctx.clone(), msg.clone(), groups.to_vec(), owners).await;

    Ok(())
}

fn help_menu(
    ses: Arc<RwLock<EmbedSession>>,
    ctx: Context,
    msg: Message,
    groups: Vec<&'static CommandGroup>,
    owners: HashSet<UserId>,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let prefix = calc_prefix(&ctx, &msg)
            .await
            .unwrap_or_else(|| ";".to_owned());

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
            .author(
                |a: &mut CreateEmbedAuthor| a.name("Help Menu").icon_url(&DEFAULT_ICON)
            ).description("Use the reactions to get the descriptions for the commands in that group.\nCurrently available commands:")
        });

        for (i, group) in groups.iter().enumerate() {
            if allowed(&ctx, &group.options, &msg, &owners).await {
                let mut cmds = String::new();

                for command in group.options.commands {
                    if allowed(&ctx, &command.options, &msg, &owners).await {
                        cmds.push_str(&format!(
                            "\n- **{}{}**",
                            &prefix,
                            command.options.names.first().expect("no command name")
                        ));
                    }
                }

                let details_ses = ses.clone();
                let details_ctx = ctx.clone();
                let details_msg = msg.clone();
                let details_owners = owners.clone();
                let all_groups = groups.clone();
                let group = *group;
                em.add_field(group.name, &cmds, true, &NUMBER_EMOJIS[i], move || {
                    let details_ses = details_ses.clone();
                    let details_ctx = details_ctx.clone();
                    let details_msg = details_msg.clone();
                    let details_owners = details_owners.clone();
                    let all_groups = all_groups.clone();
                    Box::pin(async move {
                        command_details(
                            details_ses.clone(),
                            details_ctx,
                            details_msg,
                            all_groups,
                            details_owners,
                            group,
                        )
                        .await
                    })
                });
            }
        }

        em.add_option(&ReactionType::from(EXIT_EMOJI), move || {
            let ses = ses.clone();
            Box::pin(async move {
                let lock = ses.read().await;
                if let Some(m) = lock.message.as_ref() {
                    let _ = m.delete(&lock.http).await;
                };
            })
        });

        let _ = em.show().await;
    })
}

fn command_details(
    ses: Arc<RwLock<EmbedSession>>,
    ctx: Context,
    msg: Message,
    groups: Vec<&'static CommandGroup>,
    owners: HashSet<UserId>,
    selected_group: &'static CommandGroup,
) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let prefix = calc_prefix(&ctx, &msg)
            .await
            .unwrap_or_else(|| ";".to_owned());

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name(format!("{} Commands", selected_group.name))
                        .icon_url(&DEFAULT_ICON)
                })
                .description("More Detailed information about the commands in this group")
        });

        for command in selected_group.options.commands {
            if allowed(&ctx, &command.options, &msg, &owners).await {
                let aliases: Vec<&str> = command.options.names.iter().skip(1).copied().collect();
                let aliases = if aliases.is_empty() {
                    None
                } else {
                    Some(aliases)
                };

                em.inner.field(
                    command
                        .options
                        .names
                        .first()
                        .map(|s| format!("{}{}", &prefix, s))
                        .expect("no command name"),
                    format!(
                        "{}{}{}",
                        aliases.map_or("".to_owned(), |a| format!("**Aliases**: {}", a.join(", "))),
                        command
                            .options
                            .desc
                            .map_or("".to_owned(), |d| format!("\n**Description:** {}", d)),
                        command
                            .options
                            .usage
                            .map_or("".to_owned(), |u| format!("\n{}", u))
                    ),
                    false,
                );
            }
        }

        em.add_field(
            "Back",
            "Back to help menu",
            false,
            &ReactionType::Unicode("◀️".into()),
            move || {
                let back_ses = ses.clone();
                let back_ctx = ctx.clone();
                let back_msg = msg.clone();
                let back_groups = groups.clone();
                let back_owners = owners.clone();
                Box::pin(async move {
                    help_menu(back_ses, back_ctx, back_msg, back_groups, back_owners).await
                })
            },
        );

        let _ = em.show().await;
    })
}

async fn allowed(
    ctx: &Context,
    options: &impl CommonOptions,
    msg: &Message,
    owners: &HashSet<UserId>,
) -> bool {
    if options.owners_only() && !owners.contains(&msg.author.id) {
        return false;
    }

    if options.only_in() == OnlyIn::Dm && !msg.is_private() {
        return false;
    }

    if options.only_in() == OnlyIn::Guild && msg.is_private() {
        return false;
    }

    let req_perms = options.required_permissions();

    if !req_perms.is_empty() {
        if let Some(Channel::Guild(channel)) = msg.channel(&ctx.cache).await {
            if let Ok(perms) = channel
                .permissions_for_user(&ctx.cache, msg.author.id)
                .await
            {
                if !perms.contains(*req_perms) {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    return true;
}

#[hook]
pub async fn calc_prefix(ctx: &Context, msg: &Message) -> Option<String> {
    msg.guild_id?;

    let db = if let Some(db) = ctx.data.read().await.get::<DatabaseKey>() {
        db.clone()
    } else {
        println!("No database found");
        return None;
    };

    let res = db
        .collection("general_settings")
        .find_one(doc! { "guild": msg.guild_id.unwrap().0 }, None)
        .await;

    if res.is_err() {
        println!("Error in getting prefix: {:?}", res.unwrap_err());
        return None;
    }

    res.unwrap()
        .and_then(|c| {
            let settings = bson::from_bson::<GuildSettings>(c.into());
            if settings.is_err() {
                println!("Error in getting prefix: {:?}", settings.unwrap_err());
                return None;
            }
            let settings = settings.unwrap();
            Some(settings)
        })
        .map(|s| s.prefix)
}

mod pages;
mod settings;

use std::sync::Arc;

use chrono::Utc;
use okto_framework::macros::command;
use pages::{
    filters_page,
    mentions_page,
    other_page,
    reminders_page,
};
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedAuthor,
        CreateInteractionResponse,
    },
    framework::standard::CommandResult,
    model::application::{
        component::ButtonStyle,
        interaction::{
            application_command::{
                ApplicationCommandInteraction,
                CommandDataOptionValue,
            },
            MessageFlags,
        },
    },
    prelude::{
        Context,
        RwLock,
    },
};

use crate::{
    events::statefulembed::{
        ButtonType,
        EmbedSession,
        StatefulEmbed,
    },
    utils::{
        constants::*,
        default_embed,
        reminders::ID,
        StandardButton,
    },
};

#[command]
#[default_permission(false)]
#[options(
    {
        option_type: Channel,
        name: "target_channel",
        description: "Channel to set reminders for instead of channel this command was ran in"
    }
)]
/// Manage the reminders and notifications posted by the bot in this server
async fn notifychannel(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> CommandResult {
    if interaction
        .guild_id
        .is_none()
    {
        interaction
            .create_interaction_response(
                &ctx.http,
                |m: &mut CreateInteractionResponse| {
                    m.interaction_response_data(|c| {
                        c.flags(MessageFlags::EPHEMERAL)
                            .embed(|e: &mut CreateEmbed| {
                                default_embed(
                                    e,
                                    "This command can only be ran in a server.",
                                    false,
                                )
                            })
                    })
                },
            )
            .await?;

        return Ok(());
    }

    let target_channel = if let Some(channel_id) = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "target_channel")
    {
        channel_id
            .resolved
            .clone()
            .and_then(|v| {
                if let CommandDataOptionValue::Channel(c) = v {
                    Some(c.id)
                } else {
                    None
                }
            })
            .ok_or("Invalid argument given")?
            .to_channel_cached(ctx)
            .map_or(interaction.channel_id, |channel| {
                channel.id()
            })
    } else {
        interaction.channel_id
    };

    let ses = EmbedSession::new(ctx, interaction.clone(), false).await?;

    main_menu(
        ses,
        ID::Channel((
            target_channel,
            interaction
                .guild_id
                .unwrap(),
        )),
    )
    .await;

    Ok(())
}

#[command]
/// Setup reminders and notifications from the bot in your DMs
async fn notifyme(ctx: &Context, interaction: &ApplicationCommandInteraction) -> CommandResult {
    let ses = EmbedSession::new(ctx, interaction.clone(), true).await?;

    main_menu(
        ses,
        ID::User(
            interaction
                .user
                .id,
        ),
    )
    .await;

    Ok(())
}

// -------

fn main_menu(ses: Arc<RwLock<EmbedSession>>, id: ID) -> futures::future::BoxFuture<'static, ()> {
    Box::pin(async move {
        let name = if let ID::Channel((channel, _)) = id {
            format!(
                "Launch Reminder Settings for {}",
                channel
                    .name(
                        &ses.read()
                            .await
                            .cache
                    )
                    .await
                    .map_or("guild channel".to_string(), |n| {
                        "#".to_owned() + &n
                    })
            )
        } else {
            "Launch Reminder Settings for your DMs".to_owned()
        };

        let mut em = StatefulEmbed::new_with(ses.clone(), |e: &mut CreateEmbed| {
            e.color(DEFAULT_COLOR)
                .timestamp(Utc::now())
                .author(|a: &mut CreateEmbedAuthor| {
                    a.name(name)
                        .icon_url(DEFAULT_ICON)
                })
        });

        let reminder_ses = ses.clone();
        em.add_field(
            "Reminders",
            "Set at which times you want to get launch reminders",
            false,
            &ButtonType {
                emoji: Some('⏰'.into()),
                style: ButtonStyle::Primary,
                label: "Reminders".to_owned(),
            },
            move |_| {
                let reminder_ses = reminder_ses.clone();
                Box::pin(async move { reminders_page(reminder_ses.clone(), id).await })
            },
        );

        let filters_ses = ses.clone();
        em.add_field(
            "Filters",
            "Set filters for which launches you do and don't want to see",
            false,
            &ButtonType {
                emoji: Some('🔍'.into()),
                style: ButtonStyle::Primary,
                label: "Filters".to_owned(),
            },
            move |_| {
                let filters_ses = filters_ses.clone();
                Box::pin(async move { filters_page(filters_ses.clone(), id).await })
            },
        );

        if id.guild_specific() {
            let mention_ses = ses.clone();
            em.add_field(
                "Mentions",
                "Set which roles should be mentioned when posting reminders",
                false,
                &ButtonType {
                    emoji: Some('🔔'.into()),
                    style: ButtonStyle::Primary,
                    label: "Mentions".to_owned(),
                },
                move |_| {
                    let mention_ses = mention_ses.clone();
                    Box::pin(async move { mentions_page(mention_ses.clone(), id).await })
                },
            );
        }

        let other_ses = ses.clone();
        em.add_field(
            "Other",
            "Enable other notifications",
            false,
            &ButtonType {
                emoji: Some('🛎'.into()),
                style: ButtonStyle::Primary,
                label: "Other".to_owned(),
            },
            move |_| {
                let other_ses = other_ses.clone();
                Box::pin(async move { other_page(other_ses.clone(), id).await })
            },
        );

        if id.guild_specific() {
            let close_ses = ses.clone();
            em.add_field(
                "Close",
                "Close this menu",
                false,
                &StandardButton::Exit.to_button(),
                move |_| {
                    let close_ses = close_ses.clone();
                    Box::pin(async move {
                        let lock = close_ses
                            .read()
                            .await;
                        let r = lock
                            .interaction
                            .delete_original_interaction_response(&lock.http)
                            .await;
                        if let Err(e) = r {
                            dbg!(e);
                        }
                    })
                },
            );
        }

        let result = em
            .show()
            .await;
        if let Err(err) = result {
            dbg!(err);
        }
    })
}

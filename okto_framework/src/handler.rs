use std::collections::HashMap;

use serde::Serialize;
use serenity::{
    client::Context,
    framework::standard::CommandResult,
    http::Http,
    model::{
        application::{
            command::CommandPermissionType,
            interaction::Interaction,
        },
        id::CommandId,
        Permissions,
    },
    Result,
};

use crate::{
    structs::{
        Command,
        CommandDetails,
    },
    utils::{
        get_all_guilds,
        get_roles_with_permission,
    },
};

#[derive(Clone, Debug)]
struct DiscordCommand {
    id: CommandId,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct CommandPermission {
    id: CommandId,
    permissions: Vec<CommandPermissionEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct CommandPermissionEntry {
    id: u64,
    #[serde(rename = "type")]
    kind: CommandPermissionType,
    permission: bool,
}

#[derive(Clone)]
pub struct Handler {
    cmds: HashMap<String, &'static Command>,
    d_cmds: Vec<DiscordCommand>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            cmds: HashMap::new(),
            d_cmds: Vec::new(),
        }
    }

    pub fn get_command_list(&self) -> Vec<&'static Command> {
        self.cmds
            .values()
            .copied()
            .collect()
    }

    pub fn add_command(&mut self, cmd: &'static Command) -> std::result::Result<(), String> {
        if cmd
            .options
            .description
            .is_empty()
        {
            return Err(format!(
                "Command {} has no description",
                &cmd.options
                    .name
            ));
        } else if cmd
            .options
            .description
            .len()
            > 100
        {
            return Err(format!(
                "Command {} has a description longer than 100 characters",
                &cmd.options
                    .name
            ));
        }

        self.cmds
            .insert(
                cmd.options
                    .name
                    .to_owned(),
                cmd,
            );

        Ok(())
    }

    pub async fn handle_interaction(
        &self,
        ctx: &Context,
        interaction: &Interaction,
    ) -> CommandResult {
        if let Interaction::ApplicationCommand(cmd_interaction) = interaction {
            if let Some(cmd) = self
                .cmds
                .get(
                    &cmd_interaction
                        .data
                        .name,
                )
            {
                return (cmd.func)(ctx, cmd_interaction).await;
            }
        }

        Ok(())
    }

    pub async fn upload_commands(&mut self, http: impl AsRef<Http>) -> Result<()> {
        let body = serde_json::to_value(
            &self
                .cmds
                .values()
                .map(|c| c.options)
                .collect::<Vec<&CommandDetails>>(),
        )?;

        self.d_cmds = http
            .as_ref()
            .create_global_application_commands(&body)
            .await?
            .into_iter()
            .map(|c| {
                DiscordCommand {
                    id: c.id,
                    name: c.name,
                }
            })
            .collect();

        Ok(())
    }

    pub async fn upload_permissions(&self, http: impl AsRef<Http>) -> Result<()> {
        let http = http.as_ref();
        let guilds = get_all_guilds(http).await?;

        for g in guilds {
            let mut permissions = Vec::new();

            for c in &self.d_cmds {
                if let Some(cmd) = self
                    .cmds
                    .get(&c.name)
                {
                    if cmd
                        .perms
                        .is_empty()
                    {
                        continue;
                    }

                    let discord_perms = cmd
                        .perms
                        .iter()
                        .fold(Permissions::empty(), |acc, p| {
                            acc.union(*p)
                        });

                    let mut c_perms = Vec::new();
                    for role in get_roles_with_permission(&g, discord_perms) {
                        c_perms.push(CommandPermissionEntry {
                            kind: CommandPermissionType::Role,
                            permission: true,
                            id: role
                                .id
                                .0,
                        })
                    }

                    c_perms.push(CommandPermissionEntry {
                        id: g
                            .owner_id
                            .0,
                        permission: true,
                        kind: CommandPermissionType::User,
                    });
                    if !c_perms.is_empty() {
                        permissions.push(CommandPermission {
                            id: c.id,
                            permissions: c_perms,
                        });
                    }
                }
            }

            if permissions.is_empty() {
                break;
            }

            let body = serde_json::to_value(permissions)?;

            http.edit_guild_application_commands_permissions(g.id.0, &body)
                .await?;
        }

        Ok(())
    }
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

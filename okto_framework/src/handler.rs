use std::collections::HashMap;

use serenity::{
    client::Context,
    framework::standard::CommandResult,
    http::Http,
    model::{
        application::interaction::Interaction,
    },
    Result,
};

use crate::{
    structs::{
        Command,
        DiscordCommandDetails,
    }
};

#[derive(Clone)]
pub struct Handler {
    cmds: HashMap<String, &'static Command>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            cmds: HashMap::new(),
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

    pub async fn upload_commands(&self, http: impl AsRef<Http>) -> Result<()> {
        let body = serde_json::to_value(
            &self
                .cmds
                .values()
                .map(|c| c.options.clone())
                .map(DiscordCommandDetails::from)
                .collect::<Vec<_>>(),
        )?;

        http
            .as_ref()
            .create_global_application_commands(&body)
            .await?;

        Ok(())
    }
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

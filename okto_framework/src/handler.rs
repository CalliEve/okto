use serenity::{
    client::Context,
    framework::standard::CommandResult,
    http::Http,
    model::interactions::Interaction,
    Result,
};

use crate::structs::{
    Command,
    CommandDetails,
};

pub struct Handler {
    cmds: Vec<Command>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            cmds: Vec::new(),
        }
    }

    pub fn add_command(&mut self, cmd: Command) {
        self.cmds
            .push(cmd)
    }

    pub async fn handle_interaction(
        &self,
        ctx: &Context,
        interaction: &Interaction,
    ) -> CommandResult {
        if let Interaction::ApplicationCommand(cmd_interaction) = interaction {
            for cmd in &self.cmds {
                if cmd
                    .options
                    .name
                    == cmd_interaction
                        .data
                        .name
                {
                    return (cmd.func)(ctx, cmd_interaction).await;
                }
            }
        }
        return Ok(());
    }

    pub async fn upload_commands(&self, http: impl AsRef<Http>) -> Result<()> {
        let body = serde_json::to_value(
            &self
                .cmds
                .iter()
                .map(|c| c.options)
                .collect::<Vec<&CommandDetails>>(),
        )?;

        http.as_ref()
            .create_global_application_commands(&body)
            .await?;

        Ok(())
    }
}

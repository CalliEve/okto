use futures::future::BoxFuture;
use serde::Serialize;
use serde_json::Value;
use serenity::{
    client::Context,
    framework::standard::CommandResult,
    model::{
        channel::ChannelType,
        interactions::application_command::ApplicationCommandInteraction,
    },
};

pub struct Command {
    pub options: &'static CommandDetails,
    pub func: CommandFunc,
}

pub type CommandFunc = for<'fut> fn(
    &'fut Context,
    &'fut ApplicationCommandInteraction,
) -> BoxFuture<'fut, CommandResult>;

#[derive(Debug, Clone, Serialize)]
pub struct CommandDetails {
    pub name: &'static str,
    pub description: &'static str,
    pub options: &'static [CommandOption],
    pub default_permission: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOption {
    pub option_type: CommandOptionType,
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub choices: &'static [CommandOptionChoice],
    pub channel_types: Option<&'static [ChannelType]>,
    pub min_value: Option<i32>,
    pub max_value: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOptionChoice {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum CommandType {
    ChatInput = 1,
    User = 2,
    Message = 3,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum CommandOptionType {
    SubCommand = 1,
    SubCommandGroup = 2,
    String = 3,
    Integer = 4,
    Boolean = 5,
    User = 6,
    Channel = 7,
    Role = 8,
    Mentionable = 9,
    Number = 10,
}

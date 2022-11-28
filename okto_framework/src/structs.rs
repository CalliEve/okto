use std::fmt;

use futures::future::BoxFuture;
use serde::Serialize;
use serde_repr::Serialize_repr;
use serenity::{
    client::Context,
    framework::standard::CommandResult,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        channel::ChannelType, Permissions,
    },
};

#[derive(Clone)]
pub struct Command {
    pub options: &'static CommandDetails,
    pub func: CommandFunc,
    pub info: &'static CommandInfo,
}

pub type CommandFunc = for<'fut> fn(
    &'fut Context,
    &'fut ApplicationCommandInteraction,
) -> BoxFuture<'fut, CommandResult>;

#[derive(Debug, Clone)]
pub struct CommandDetails {
    pub name: &'static str,
    pub description: &'static str,
    pub options: &'static [CommandOption],
    pub default_permission: bool,
    pub available_in_dms: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DiscordCommandDetails {
    pub name: &'static str,
    pub description: &'static str,
    pub options: &'static [CommandOption],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<Permissions>,
    pub dm_permission: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOption {
    #[serde(rename = "type")]
    pub option_type: CommandOptionType,
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub choices: Option<&'static [CommandOptionChoice]>,
    pub channel_types: Option<&'static [ChannelType]>,
    pub min_value: Option<i32>,
    pub max_value: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOptionChoice {
    pub name: &'static str,
    pub value: CommandOptionValue,
}

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub file: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum CommandOptionValue {
    String(&'static str),
    Integer(i32),
    Double(f64),
}

#[derive(Debug, Clone, Copy, Serialize_repr)]
#[repr(u8)]
pub enum CommandType {
    ChatInput = 1,
    User = 2,
    Message = 3,
}

#[derive(Debug, Clone, Copy, Serialize_repr)]
#[repr(u8)]
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

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Command")
            .field("options", self.options)
            .field("info", self.info)
            .finish()
    }
}

impl From<CommandDetails> for DiscordCommandDetails {
    fn from(c: CommandDetails) -> Self {
        DiscordCommandDetails {
            name: c.name,
            description: c.description,
            options: c.options,
            default_member_permissions: if c.default_permission {
                None
            } else {
                Some(Permissions::MANAGE_GUILD)
            },
            dm_permission: c.available_in_dms,
        }
    }
}

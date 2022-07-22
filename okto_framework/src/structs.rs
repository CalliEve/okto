use std::fmt;

use futures::future::BoxFuture;
use serde::Serialize;
use serde_repr::Serialize_repr;
use serenity::{
    client::Context,
    framework::standard::{CommandResult, OnlyIn},
    model::{
        channel::ChannelType, interactions::application_command::ApplicationCommandInteraction,
        Permissions,
    },
};

// FIXME: remove permissions as they are now

#[derive(Clone)]
pub struct Command {
    pub options: &'static CommandDetails,
    pub perms: &'static [Permissions],
    pub func: CommandFunc,
    pub info: &'static CommandInfo,
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
    pub only_in: OnlyIn,
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

impl Command {
    pub fn only_in(&self) -> OnlyIn {
        self.info
            .only_in
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Command")
            .field("options", self.options)
            .field("perms", &self.perms)
            .field("info", self.info)
            .finish()
    }
}

use std::collections::HashMap;

use proc_macro2::Span;
use quote::{
    quote,
    format_ident,
    ToTokens,
};
use syn::{
    braced,
    parenthesized,
    parse::{
        Error,
        Parse,
        ParseStream,
        Result,
    },
    punctuated::Punctuated,
    token::{
        Bracket,
        Comma,
    },
    Expr,
    Ident,
    LitBool,
    LitInt,
    LitStr,
    Token,
};

mod kw {
    syn::custom_keyword!(options);
}

pub struct ParenthesisedItems<T>(pub Punctuated<T, Comma>);

impl<T: Parse> Parse for ParenthesisedItems<T> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        parenthesized!(content in input);
        Ok(Self(content.parse_terminated(T::parse)?))
    }
}

pub enum CommandAttributeContent {
    String(String),
    Boolean(bool),
    Options(Vec<CommandOption>),
}

impl CommandAttributeContent {
    pub fn get_string(self) -> Result<String> {
        match self {
            Self::String(s) => Ok(s),
            _ => Err(Error::new(Span::call_site(), "invalid command attribute")),
        }
    }

    pub fn get_boolean(self) -> Result<bool> {
        match self {
            Self::Boolean(b) => Ok(b),
            _ => Err(Error::new(Span::call_site(), "invalid command attribute")),
        }
    }

    pub fn get_options(self) -> Result<Vec<CommandOption>> {
        match self {
            Self::Options(o) => Ok(o),
            _ => Err(Error::new(Span::call_site(), "invalid command attribute")),
        }
    }
}

impl Parse for CommandAttributeContent {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitBool) {
            return Ok(Self::Boolean(
                input
                    .parse::<LitBool>()?
                    .value,
            ));
        } else if lookahead.peek(LitStr) {
            return Ok(Self::String(
                input
                    .parse::<LitStr>()?
                    .value().trim().to_owned(),
            ));
        } else if lookahead.peek(Bracket) {
            let content;
            syn::bracketed!(content in input);
            return Ok(Self::Options(
                content
                    .parse_terminated::<_, Token![,]>(CommandOption::parse)?
                    .into_iter()
                    .collect(),
            ));
        }

        Err(Error::new_spanned(
            input.to_string(),
            "Not a command attribute",
        ))
    }
}

macro_rules! get_field {
    ($name:literal, $map:expr, $res:ident) => {{
        match $map.get($name) {
            Some(value) => syn::parse::<$res>(
                value
                    .into_token_stream()
                    .into(),
            )?,
            None => {
                return Err(Error::new(
                    Span::call_site(),
                    format!("No required field {} in command attributes", $name),
                ))
            },
        }
    }};
    (false, $name:literal, $map:expr, $res:ident) => {{
        match $map.get($name) {
            Some(value) => Some(syn::parse::<$res>(
                value
                    .into_token_stream()
                    .into(),
            )?),
            None => None,
        }
    }};
}

pub struct CommandOption {
    pub option_type: CommandOptionType,
    pub name: String,
    pub description: String,
    pub required: bool,
    //pub choices: Vec<CommandOptionChoice>,
    //pub channel_types: Option<Vec<ChannelType>>,
    pub min_value: Option<i32>,
    pub max_value: Option<i32>,
}

impl Parse for CommandOption {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        braced!(content in input);
        let fields: HashMap<String, Expr> = content
            .parse_terminated::<_, Token![,]>(StructField::parse)?
            .into_iter()
            .map(|f| (f.name, f.value))
            .collect();

        Ok(Self {
            option_type: get_field!("type", fields, CommandOptionType),
            name: get_field!("name", fields, LitStr).value(),
            description: get_field!("description", fields, LitStr).value(),
            required: get_field!(false, "required", fields, LitBool).map_or(false, |v| v.value),
            min_value: get_field!(false, "min_value", fields, LitInt).map(|v| {
                v.base10_parse()
                    .unwrap()
            }),
            max_value: get_field!(false, "max_value", fields, LitInt).map(|v| {
                v.base10_parse()
                    .unwrap()
            }),
        })
    }
}

impl ToTokens for CommandOption {
    fn to_tokens(&self, stream: &mut proc_macro2::TokenStream) {
        let Self {
            option_type,
            name,
            description,
            required,
            min_value,
            max_value,
        } = self.clone();
        let min_value = tokenize_option(*min_value);
        let max_value = tokenize_option(*max_value);

        stream.extend(quote! {
            CommandOption {
                name: #name,
                description: #description,
                option_type: #option_type,
                required: #required,
                min_value: #min_value,
                max_value: #max_value
            }
        })
    }
}

struct StructField {
    name: String,
    value: Expr,
}

impl Parse for StructField {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input
            .parse::<Ident>()?
            .to_string();
        input.parse::<Token![:]>()?;
        let value = input.parse::<Expr>()?;
        Ok(Self {
            name,
            value,
        })
    }
}

pub enum CommandOptionType {
    SubCommand,
    SubCommandGroup,
    String,
    Integer,
    Boolean,
    User,
    Channel,
    Role,
    Mentionable,
    Number,
}

impl Parse for CommandOptionType {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(
            match input
                .parse::<Ident>()?
                .to_string()
                .as_str()
            {
                "SubCommand" => Self::SubCommand,
                "SubCommandGroup" => Self::SubCommandGroup,
                "String" => Self::String,
                "Integer" => Self::Integer,
                "Boolean" => Self::Boolean,
                "User" => Self::User,
                "Channel" => Self::Channel,
                "Role" => Self::Role,
                "Mentionable" => Self::Mentionable,
                "Number" => Self::Number,
                t => {
                    return Err(Error::new(
                        Span::call_site(),
                        format!("invalid option type: {}", t),
                    ))
                },
            },
        )
    }
}

impl ToTokens for CommandOptionType {
    fn to_tokens(&self, stream: &mut proc_macro2::TokenStream) {
        let ident = Ident::new(match self {
            &Self::SubCommand => "SubCommand",
            &Self::SubCommandGroup => "SubCommandGroup",
            &Self::String => "String",
            &Self::Integer => "Integer",
            &Self::Boolean => "Boolean",
            &Self::User => "User",
            &Self::Channel => "Channel",
            &Self::Role => "Role",
            &Self::Mentionable => "Mentionable",
            &Self::Number => "Number"
        }, Span::call_site());

        stream.extend(quote! {#ident})
    }
}

pub fn into_stream(e: Error) -> proc_macro2::TokenStream {
    e.into_compile_error()
}

pub fn add_suffix(ident: &Ident, suffix: &str) -> Ident {
    format_ident!("{}_{}", ident.to_string(), suffix)
}

pub fn tokenize_option<T: ToTokens>(opt: Option<T>) -> proc_macro2::TokenStream {
    if let Some(t) = opt {
        quote! {Some(#t)}
    } else {
        quote! {None}
    }
}


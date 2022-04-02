use std::collections::HashMap;

use proc_macro2::Span;
use quote::{
    format_ident,
    quote,
    ToTokens,
};
use syn::{
    braced,
    bracketed,
    parenthesized,
    parse::{
        Error,
        Parse,
        ParseStream,
        Result,
    },
    punctuated::Punctuated,
    token::{
        Brace,
        Bracket,
        Comma,
    },
    Expr,
    Ident,
    LitBool,
    LitFloat,
    LitInt,
    LitStr,
    Path,
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
    Permissions(Vec<Path>),
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

    pub fn get_permissions(self) -> Result<Vec<Path>> {
        match self {
            Self::Permissions(o) => Ok(o),
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
                    .value()
                    .trim()
                    .to_owned(),
            ));
        } else if lookahead.peek(Ident) {
            return Ok(Self::Permissions(
                input
                    .parse_terminated::<_, Token![,]>(Ident::parse)?
                    .into_iter()
                    .map(|p| {
                        syn::parse_str::<Path>(&format!(
                            "::serenity::model::Permissions::{}",
                            p.to_string()
                                .to_uppercase()
                        ))
                        .expect("permissions ident is invalid")
                    })
                    .collect(),
            ));
        } else if lookahead.peek(Brace) {
            return Ok(Self::Options(
                input
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
    ($name:literal, $map:expr, $res:ty) => {{
        match $map.get($name) {
            Some(value) => syn::parse2::<$res>(
                value
                    .into_token_stream(),
            )?,
            None => {
                return Err(Error::new(
                    Span::call_site(),
                    format!("No required field {} in command attributes", $name),
                ))
            },
        }
    }};
    (false, $name:literal, $map:expr, $res:ty) => {{
        match $map.get($name) {
            Some(value) => Some(syn::parse2::<$res>(
                value
                    .into_token_stream(),
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
    pub choices: Option<List<CommandOptionChoice>>,
    //pub channel_types: Option<Vec<ChannelType>>,
    pub min_value: Option<i32>,
    pub max_value: Option<i32>,
}

impl Parse for CommandOption {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        braced!(content in input);
        let fields: HashMap<String, StructFieldValue> = content
            .parse_terminated::<_, Token![,]>(StructField::parse)?
            .into_iter()
            .map(|f| (f.name, f.value))
            .collect();

        Ok(Self {
            option_type: get_field!("option_type", fields, CommandOptionType),
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
            choices: get_field!(false, "choices", fields, List<CommandOptionChoice>),
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
            choices,
        } = self.clone();
        let min_value = tokenize_option(*min_value);
        let max_value = tokenize_option(*max_value);
        let choices = tokenize_option(choices.clone());

        stream.extend(quote! {
            okto_framework::structs::CommandOption {
                name: #name,
                description: #description,
                option_type: #option_type,
                required: #required,
                min_value: #min_value,
                max_value: #max_value,
                choices: #choices,
                channel_types: None
            }
        })
    }
}

#[derive(Debug, Clone)]
struct StructField {
    name: String,
    value: StructFieldValue,
}

impl Parse for StructField {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input
            .parse::<Ident>()?
            .to_string();
        input.parse::<Token![:]>()?;
        let value = input.parse::<StructFieldValue>()?;
        Ok(Self {
            name,
            value,
        })
    }
}

impl ToTokens for StructField {
    fn to_tokens(&self, stream: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            value,
        } = self.clone();
        let name = Ident::new(&name, Span::call_site());

        stream.extend(quote! {#name: #value})
    }
}

#[derive(Debug, Clone)]
enum StructFieldValue {
    Expr(Expr),
    List(Punctuated<StructFieldValue, Comma>),
    Map(Punctuated<StructField, Comma>),
}

impl Parse for StructFieldValue {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Bracket) {
            let content;
            bracketed!(content in input);
            Ok(StructFieldValue::List(
                content.parse_terminated::<_, Token![,]>(StructFieldValue::parse)?,
            ))
        } else if lookahead.peek(Brace) {
            let content;
            braced!(content in input);
            Ok(StructFieldValue::Map(
                content.parse_terminated::<_, Token![,]>(StructField::parse)?,
            ))
        } else {
            Ok(StructFieldValue::Expr(input.parse()?))
        }
    }
}

impl ToTokens for StructFieldValue {
    fn to_tokens(&self, stream: &mut proc_macro2::TokenStream) {
        stream.extend(match self {
            Self::Expr(e) => quote! {#e},
            Self::List(l) => quote! {[#l]},
            Self::Map(m) => quote! {{#m}},
        })
    }
}

#[derive(Debug, Clone)]
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
        let ident = Ident::new(
            match self {
                &Self::SubCommand => "SubCommand",
                &Self::SubCommandGroup => "SubCommandGroup",
                &Self::String => "String",
                &Self::Integer => "Integer",
                &Self::Boolean => "Boolean",
                &Self::User => "User",
                &Self::Channel => "Channel",
                &Self::Role => "Role",
                &Self::Mentionable => "Mentionable",
                &Self::Number => "Number",
            },
            Span::call_site(),
        );

        stream.extend(quote! {okto_framework::structs::CommandOptionType::#ident})
    }
}

#[derive(Debug, Clone)]
pub struct CommandOptionChoice {
    pub name: String,
    pub value: Value,
}

impl Parse for CommandOptionChoice {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        braced!(content in input);
        let fields: HashMap<String, StructFieldValue> = content
            .parse_terminated::<_, Token![,]>(StructField::parse)?
            .into_iter()
            .map(|f| (f.name, f.value))
            .collect();
        Ok(Self {
            name: get_field!("name", fields, LitStr).value(),
            value: get_field!("value", fields, Value),
        })
    }
}

impl ToTokens for CommandOptionChoice {
    fn to_tokens(&self, stream: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            value,
        } = self.clone();

        stream.extend(quote! {
            okto_framework::structs::CommandOptionChoice {
                name: #name,
                value: #value
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i32),
    Double(f64),
}

impl Parse for Value {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            Ok(Self::String(
                input
                    .parse::<LitStr>()?
                    .value(),
            ))
        } else if lookahead.peek(LitInt) {
            Ok(Self::Integer(
                input
                    .parse::<LitInt>()?
                    .base10_parse()?,
            ))
        } else if lookahead.peek(LitFloat) {
            Ok(Self::Double(
                input
                    .parse::<LitFloat>()?
                    .base10_parse()?,
            ))
        } else {
            Err(Error::new_spanned(
                input.to_string(),
                "Not a valid command option choice value",
            ))
        }
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, stream: &mut proc_macro2::TokenStream) {
        stream.extend(match self {
            Self::String(s) => {
                quote! {okto_framework::structs::CommandOptionValue::String(#s)}
            },
            Self::Integer(i) => {
                quote! {okto_framework::structs::CommandOptionValue::Integer(#i)}
            },
            Self::Double(d) => {
                quote! {okto_framework::structs::CommandOptionValue::Double(#d)}
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct List<T> {
    inner: Punctuated<T, Comma>,
}

impl<T: Parse> Parse for List<T> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        bracketed!(content in input);
        Ok(List {
            inner: content.parse_terminated::<_, Token![,]>(T::parse)?,
        })
    }
}

impl<T: ToTokens> ToTokens for List<T> {
    fn to_tokens(&self, stream: &mut proc_macro2::TokenStream) {
        let inner = &self.inner;
        stream.extend(quote! {&[#inner]});
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

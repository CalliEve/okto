mod structs;
mod utils;

#[allow(unused_extern_crates)]
extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse2,
    Path,
};
use utils::{
    add_suffix,
    CommandAttributeContent,
};

use crate::structs::CommandFunc;

macro_rules! propagate_err {
    ($res:expr) => {{
        match $res {
            Ok(v) => v,
            Err(e) => return $crate::utils::into_stream(e),
        }
    }};
}

#[proc_macro_attribute]
pub fn command(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    command_inner(item.into()).into()
}

fn command_inner(item: TokenStream) -> TokenStream {
    let command_fun: CommandFunc = propagate_err!(parse2(item));

    let mut command_name = command_fun
        .name
        .to_string();
    let mut description = String::new();
    let mut options = Vec::new();
    let mut default_permission = true;
    let mut required_permissions: Vec<Path> = Vec::new();
    let mut only_in = quote! {::serenity::framework::standard::OnlyIn::None};

    for attr in command_fun
        .attributes
        .clone()
    {
        if let Some(p) = attr
            .path
            .get_ident()
        {
            match p
                .to_string()
                .as_str()
            {
                "name" => {
                    command_name = propagate_err!(propagate_err!(
                        attr.parse_args::<CommandAttributeContent>()
                    )
                    .get_string())
                },
                "description" => {
                    description = description
                        + " "
                        + &propagate_err!(propagate_err!(
                            attr.parse_args::<CommandAttributeContent>()
                        )
                        .get_string())
                },
                "default_permission" => {
                    default_permission = propagate_err!(propagate_err!(
                        attr.parse_args::<CommandAttributeContent>()
                    )
                    .get_boolean())
                },
                "options" => {
                    options = propagate_err!(propagate_err!(
                        attr.parse_args::<CommandAttributeContent>()
                    )
                    .get_options())
                },
                "required_permissions" => {
                    required_permissions = propagate_err!(propagate_err!(attr
                        .parse_args::<CommandAttributeContent>(
                    ))
                    .get_permissions())
                },
                "only_in" => {
                    let tmp = propagate_err!(propagate_err!(
                        attr.parse_args::<CommandAttributeContent>()
                    )
                    .get_string());
                    match tmp.as_str() {
                        "Dm" => only_in = quote! {::serenity::framework::standard::OnlyIn::Dm},
                        "Guild" => {
                            only_in = quote! {::serenity::framework::standard::OnlyIn::Guild}
                        },
                        _ => {
                            return quote! {compile_error!("The value of only_in can only be `Dm` or `Guild`");};
                        },
                    }
                },
                _ => (),
            }
        }
    }

    if description.is_empty() {
        let error = format!(
            "No description has been provided for the {} command",
            command_name
        );
        return quote! {compile_error!(#error);};
    } else if description.len() > 100 {
        let error = format!(
            "The description of the {} command is longer than 100 characters",
            command_name
        );
        return quote! {compile_error!(#error);};
    }

    let fun_name = command_fun
        .name
        .clone();
    let command_struct_name = add_suffix(&fun_name, "COMMAND");
    let details_struct_name = add_suffix(&fun_name, "COMMAND_DETAILS");
    let info_struct_name = add_suffix(&fun_name, "COMMAND_INFO");

    let command_cooked = command_fun
        .cooked
        .clone();
    let details_cooked = command_cooked.clone();

    let command_struct_path = quote!(okto_framework::structs::Command);
    let details_struct_path = quote!(okto_framework::structs::CommandDetails);
    let info_struct_path = quote!(okto_framework::structs::CommandInfo);

    quote! {
        #(#details_cooked)*
        pub static #details_struct_name: #details_struct_path = #details_struct_path {
            name: #command_name,
            description: #description,
            default_permission: #default_permission,
            options: &[#(#options),*]
        };

        #(#details_cooked)*
        pub static #info_struct_name: #info_struct_path = #info_struct_path {
            file: file!(),
            only_in: #only_in,
        };

        #(#command_cooked)*
        pub static #command_struct_name: #command_struct_path = #command_struct_path {
            options: &#details_struct_name,
            perms: &[#(#required_permissions),*],
            info: &#info_struct_name,
            func: #fun_name,
        };

        #command_fun
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let stream: TokenStream = "
        /// just testing this stuff
        #[required_permissions(MANAGE_GUILD)]
        #[options(
            {
                option_type: String,
                name: \"image-version\",
                description: \"natural or enhanced version of the image of our planet earth\",
                choices: [
                    {
                        name: \"natural\",
                        value: \"natural\"
                    },
                    {
                        name: \"enhanced\",
                        value: \"enhanced\"
                    }
                ]
            }
        )]
        async fn ping(ctx: &Context) -> Result<()> {
            ctx.reply(\"test\").await;
        }"
        .parse::<TokenStream>()
        .map_err(|e| e.to_string())
        .unwrap();

        let out = command_inner(stream);
        println!("{}", out.to_string());

        assert!(!out
            .to_string()
            .starts_with("compile_error"));
        panic!("show")
    }
}

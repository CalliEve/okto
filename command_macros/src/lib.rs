mod structs;
mod utils;

#[allow(unused_extern_crates)]
extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse2;
use utils::{
    CommandAttributeContent,
    add_suffix
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
pub fn command(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
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

    for attr in command_fun.attributes.clone() {
        if let Some(p) = attr.path.get_ident() {

        match p.to_string().as_str()
        {
            "name" => {
                command_name = propagate_err!(propagate_err!(attr.parse_args::<CommandAttributeContent>()).get_string())
            },
            "description" => {
                description = propagate_err!(propagate_err!(attr.parse_args::<CommandAttributeContent>()).get_string())
            },
            "default_permission" => {
                default_permission = propagate_err!(propagate_err!(attr.parse_args::<CommandAttributeContent>()).get_boolean())
            },
            "options" => {
                options = propagate_err!(propagate_err!(attr.parse_args::<CommandAttributeContent>()).get_options())
            },
            _ => (),
        }
        }
    }

    if description.len() < 3 {
        panic!(
            "No description longer than 3 characters has been provided for the {} command, while descriptions are required by discord",
            command_name
        )
    }

    let fun_name = command_fun
        .name
        .clone();
    let command_struct_name = add_suffix(&fun_name, "COMMAND");
    let details_struct_name = add_suffix(&fun_name, "COMMAND_DETAILS");

    let command_cooked = command_fun
        .cooked
        .clone();
    let details_cooked = command_cooked.clone();

    let command_struct_path = quote!(okto_framework::structs::Command);
    let details_struct_path = quote!(okto_framework::structs::CommandDetails);

    (quote! {
        #(#details_cooked)*
        pub static #details_struct_name: #details_struct_path = #details_struct_path {
            name: #command_name,
            description: #description,
            default_permission: #default_permission,
            options: &[#(#options),*]
        };

        #(#command_cooked)*
        pub static #command_struct_name: #command_struct_path = #command_struct_path {
            options: &#details_struct_name,
            func: #fun_name,
        };

        #command_fun
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let stream: TokenStream = "
        /// just testing this stuff
        async fn ping(ctx: &Context) -> Result<()> {
            ctx.reply(\"test\").await;
        }".parse().unwrap();

        let out = command_inner(stream);
        println!("{}", out.to_string());

        assert!(!out.to_string().starts_with("compile_error"));
    }
}


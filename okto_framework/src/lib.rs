mod handler;
pub mod structs;
mod utils;

pub mod macros {
    pub use slash_command_macros::command;
}

pub use handler::Handler;

#[macro_export]
macro_rules! create_framework {
    ($token:expr, $id:expr $(, $c:ident )*) => {
        {
            okto_framework::paste_expr! {
                let mut fr = okto_framework::Handler::new();
                $(
                     fr.add_command(&[<$c _COMMAND>]).unwrap();
                )*
                let mut http = serenity::http::Http::new_with_application_id($token, $id);
                fr.upload_commands(&http).await.expect("Can't upload commands");
                fr.upload_permissions(&http).await.expect("Can't upload command permissions");
                fr
            }
        }
    }
}

#[doc(hidden)]
#[allow(unused_imports)]
pub use paste::expr as paste_expr;

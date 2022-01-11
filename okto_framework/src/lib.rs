mod handler;
pub mod structs;
mod utils;

pub mod macros {
    pub use slash_command_macros::command;
}

pub use handler::Handler;

#[macro_export]
macro_rules! create_framework {
    ($token:expr $(, $c:expr )*) => {
        {
            okto_framework::paste_expr! {
                let mut fr = okto_framework::Handler::new();
                $(
                     fr.add_command(&(&[<$c _COMMAND>]));
                )*
                let mut http = serenity::http::Http::new_with_token($token);
                let u = http.get_current_user().await.expect("Can't get current user");
                http.application_id = u.id.0;
                fr.upload_commands(http).await.expect("Can't upload commands");
                fr
            }
        }
    }
}

#[doc(hidden)]
#[allow(unused_imports)]
pub use paste::expr as paste_expr;

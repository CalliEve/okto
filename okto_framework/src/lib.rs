mod handler;
pub mod structs;

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
                let mut http = serenity::http::Http::new($token);
                http.set_application_id($id);
                fr.upload_commands(&http).await.expect("Can't upload commands");
                fr
            }
        }
    }
}

#[doc(hidden)]
#[allow(unused_imports)]
pub use paste::expr as paste_expr;

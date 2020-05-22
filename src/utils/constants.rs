use lazy_static::lazy_static;
use reqwest::{
    blocking::{Client, ClientBuilder},
    header::{HeaderMap, ACCEPT},
};
use std::env;

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert(ACCEPT, "application/json".parse().unwrap());

    headers
}

lazy_static! {
    pub static ref DEFAULT_CLIENT: Client = ClientBuilder::new()
        .user_agent("okto-bot")
        .default_headers(default_headers())
        .build()
        .expect("reqwest client could not be built");
    pub static ref GOOGLE_KEY: String = env::var("GOOGLE_KEY").expect("no GOOGLE_KEY has been set");
    pub static ref NASA_KEY: String = env::var("NASA_KEY").expect("no NASA_KEY has been set");
}

pub const DEFAULT_COLOR: u32 = 16750899;
pub const DEFAULT_ICON: &str = "https://i.imgur.com/ruFc9fc.png";
pub const TRANSPARENT_ICON: &str = "https://i.imgur.com/L2FoV6P.png";

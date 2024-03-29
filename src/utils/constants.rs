use std::{
    collections::HashMap,
    env,
};

use lazy_static::lazy_static;
use rand::{
    rngs::StdRng,
    thread_rng,
    SeedableRng,
};
use regex::Regex;
use reqwest::{
    header::{
        HeaderMap,
        ACCEPT,
    },
    Client,
    ClientBuilder,
};
use serenity::{
    model::{
        channel::ReactionType,
        id::{
            EmojiId,
            UserId,
        },
    },
    prelude::Mutex,
};

pub const DEFAULT_COLOR: u32 = 16750899;
pub const DEFAULT_ICON: &str = "https://i.imgur.com/L2FoV6P.png";
pub const FINAL_PAGE_EMOJI: char = '⏭';
pub const NEXT_PAGE_EMOJI: char = '▶';
pub const LAST_PAGE_EMOJI: char = '◀';
pub const FIRST_PAGE_EMOJI: char = '⏮';
pub const EXIT_EMOJI: char = '\u{2716}';
pub const CHECK_EMOJI: char = '✔';
pub const BACK_EMOJI: char = '◀';
pub const CERTAIN_EMOJI: u64 = 447805610482728964;
pub const UNCERTAIN_EMOJI: u64 = 447805624923717642;
pub const LAUNCH_LIBRARY_URL: &str = "https://thespacedevs.com";

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.insert(
        ACCEPT,
        "application/json"
            .parse()
            .unwrap(),
    );

    headers
}

lazy_static! {
    pub static ref GOOGLE_KEY: String = env::var("GOOGLE_KEY").expect("no GOOGLE_KEY has been set");
    pub static ref NASA_KEY: String = env::var("NASA_KEY").expect("no NASA_KEY has been set");
    pub static ref TOPGG_TOKEN: String =
        env::var("TOPGG_TOKEN").expect("no TOPGG_TOKEN has been set");
    pub static ref LL_KEY: String = format!(
        "Token {}",
        env::var("LL_KEY").expect("no LL_KEY has been set")
    );
    pub static ref DEFAULT_CLIENT: Client = ClientBuilder::new()
        .user_agent("okto-bot")
        .default_headers(default_headers())
        .build()
        .expect("reqwest client could not be built");
    pub static ref PROGRADE: ReactionType = ReactionType::Custom {
        animated: false,
        name: Some("Prograde".to_owned()),
        id: EmojiId::new(433308892584476674),
    };
    pub static ref RETROGRADE: ReactionType = ReactionType::Custom {
        animated: false,
        name: Some("Retrograde".to_owned()),
        id: EmojiId::new(433308874343448576),
    };
    pub static ref MENTION_REGEX: Regex = Regex::new("<[@#][!&]?([0-9]{17,20})>").unwrap();
    pub static ref ID_REGEX: Regex = Regex::new("^[0-9]{17,20}$").unwrap();
    pub static ref WORD_REGEX: Regex = Regex::new(r"^[a-zA-Z\-_0-9]+$").unwrap();
    pub static ref WORD_FILTER_REGEX: Regex =
        Regex::new(r"^\(\?i\)\\b[a-zA-Z\-_0-9]+\\b$").unwrap();
    pub static ref NUMBER_EMOJIS: Vec<ReactionType> = [
        "1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟"
    ]
    .iter()
    .map(|e| ReactionType::Unicode((*e).to_string()))
    .collect();
    pub static ref OWNERS: Vec<UserId> = vec![247745860979392512.into()];
}

fn agency_map() -> HashMap<&'static str, &'static str> {
    let mut res = HashMap::with_capacity(25);

    res.insert("blueorigin", "Blue Origin");
    res.insert(
        "khsc",
        "Khrunichev State Research and Production Space Center",
    );
    res.insert("spacex", "SpaceX");
    res.insert("virginorbit", "Virgin Orbit");
    res.insert("vector", "Vector");
    res.insert("isck", "ISC Kosmotras");
    res.insert("ils", "International Launch Services");
    res.insert("iai", "Israel Aerospace Industries");
    res.insert(
        "nces",
        "National Center of Space Research",
    );
    res.insert(
        "kcst",
        "Korean Committee of Space Technology",
    );
    res.insert("ula", "United Launch Alliance");
    res.insert(
        "isro",
        "Indian Space Research Organization",
    );
    res.insert("isa", "Israeli Space Agency");
    res.insert(
        "jaxa",
        "Japan Aerospace Exploration Agency",
    );
    res.insert(
        "nasa",
        "National Aeronautics and Space Administration",
    );
    res.insert(
        "roscosmos",
        "Russian Federal Space Agency (ROSCOSMOS)",
    );
    res.insert("mhi", "Mitsubishi Heavy Industries");
    res.insert("arianespace", "Arianespace");
    res.insert("eurockot", "Eurockot Launch Services");
    res.insert("rocketlab", "Rocket Lab Ltd");
    res.insert("relativity", "Relativity Space");
    res.insert(
        "ngis",
        "Northrop Grumman Innovation Systems",
    );
    res.insert(
        "casc",
        "China Aerospace Science and Technology Corporation",
    );
    res.insert(
        "casic",
        "China Aerospace Science and Industry Corporation",
    );
    res.insert(
        "cnsa",
        "China National Space Administration",
    );
    res.insert("galactic-energy", "Galactic Energy");
    res.insert("landspace", "LandSpace");
    res.insert("astra", "Astra Space");
    res.insert("firefly", "Firefly Aerospace");
    res.insert("abl", "ABL Space Systems");
    res.insert("expace", "ExPace");
    res.insert("rsf", "Russian Space Forces");

    res
}

fn vehicle_map() -> HashMap<&'static str, Vec<&'static str>> {
    let mut res = HashMap::with_capacity(25);

    res.insert("electron", vec!["Electron"]);
    res.insert(
        "falcon",
        vec![
            "Falcon Heavy",
            "Falcon 9 Full Thrust",
            "Falcon 9 v1.1",
            "Falcon 9 Block 5",
        ],
    );
    res.insert(
        "angara",
        vec![
            "Angara-1.2pp",
            "Angara A5/Briz-M",
            "Angara 1.2",
        ],
    );
    res.insert("astra", vec!["Astra Rocket 3"]);
    res.insert("falconheavy", vec!["Falcon Heavy"]);
    res.insert(
        "soyuz",
        vec![
            "Soyuz 2.1B",
            "Soyuz-FG",
            "Soyuz 2.1A",
            "Soyuz STB/Fregat",
            "Soyuz 2-1v/Volga",
            "Soyuz 2.1B/Fregat",
            "Soyuz 2.1A/Volga",
            "Soyuz STA/Fregat",
            "Soyuz 2.1A/Fregat",
            "Soyuz STB/Fregat-MT",
            "Soyuz-FG/Fregat",
            "Soyuz-U2",
            "Soyuz 2.1B/Fregat-M",
        ],
    );
    res.insert(
        "atlas5",
        vec![
            "Atlas V 551",
            "Atlas V 541",
            "Atlas V 531",
            "Atlas V 521",
            "Atlas V 511",
            "Atlas V 501",
            "Atlas V 401",
            "Atlas V 411",
            "Atlas V 421",
            "Atlas V 431",
        ],
    );
    res.insert(
        "delta4",
        vec![
            "Delta IV Heavy",
            "Delta IV",
            "Delta IV M+(4,2)",
            "Delta IV M+(5,2)",
            "Delta IV M+(5,4)",
        ],
    );
    res.insert(
        "delta2",
        vec![
            "Delta II 7320",
            "Delta II 7920H",
            "Delta II 7920-10",
            "Delta II 7420-10",
            "Delta II 7920-10C",
            "Delta II 7925-10C",
            "Delta II 7925",
            "Delta II 7925H-9.5",
            "Delta II 7925-9.5",
            "Delta II 7920H-10C",
            "Delta II 7420",
            "Delta II 7925-10L",
        ],
    );
    res.insert(
        "minotaur",
        vec!["Minotaur I", "Minotaur V"],
    );
    res.insert("pegasus", vec!["Pegasus XL"]);
    res.insert(
        "ariane5",
        vec!["Ariane 5 ES", "Ariane 5 ECA"],
    );
    res.insert("h-iib", vec!["H-IIB"]);
    res.insert(
        "zenit",
        vec!["Zenit 3SL", "Zenit 3F", "Zenit 3SLB"],
    );
    res.insert(
        "pslv",
        vec!["PSLV", "PSLV XL", "PSLV-CA"],
    );
    res.insert(
        "rokot",
        vec!["Rokot", "Rokot / Briz-KM"],
    );
    res.insert("gslv", vec!["GSLV"]);
    res.insert("vega", vec!["VEGA"]);
    res.insert(
        "antares",
        vec![
            "Antares 110",
            "Antares 120",
            "Antares 130",
            "Antares 230",
        ],
    );
    res.insert("epsilon", vec!["Epsilon"]);
    res.insert("proton", vec!["Proton-M/Briz-M"]);

    res.insert(
        "long-march2",
        vec![
            "Long March 2C",
            "Long March 2D",
            "Long March 2F",
            "Long March 2F/G",
            "Long March 2C/YZ-1S",
        ],
    );
    res.insert("long-march3", vec!["Long March 3B/E"]);
    res.insert(
        "long-march4",
        vec!["Long March 4B", "Long March 4C"],
    );
    res.insert(
        "long-march5",
        vec!["Long March 5", "Long March 5B"],
    );
    res.insert(
        "long-march6",
        vec!["Long March 6", "Long March 6A"],
    );
    res.insert(
        "long-march7",
        vec!["Long March 7", "Long March 7A"],
    );
    res.insert("long-march8", vec!["Long March 8"]);
    res.insert("long-march11", vec!["Long March 11"]);
    res.insert("firefly", vec!["Firefly Alpha"]);
    res.insert("starship", vec!["Starship"]);
    res.insert(
        "vulcan",
        vec![
            "Vulcan",
            "Vulcan VC6L",
            "Vulcan VC4L",
            "Vulcan VC2S",
        ],
    );

    res
}

lazy_static! {
    pub static ref LAUNCH_AGENCIES: HashMap<&'static str, &'static str> = agency_map();
    pub static ref LAUNCH_VEHICLES: HashMap<&'static str, Vec<&'static str>> = vehicle_map();
    pub static ref RNG: Mutex<StdRng> = Mutex::new(StdRng::from_rng(thread_rng()).unwrap());
}

use chrono::NaiveDateTime;
use serde::{self, de::Error, Deserialize, Deserializer, Serializer};

const FORMAT: &str = "%B %-e, %Y %H:%M:%S UTC";

pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let date_str = date.format(FORMAT).to_string();
    serializer.serialize_str(&date_str)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let string: String = String::deserialize(deserializer)?;

    Ok(NaiveDateTime::parse_from_str(&string, FORMAT).map_err(Error::custom)?)
}

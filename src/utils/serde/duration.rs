use chrono::Duration;
use serde::{self, Deserialize, Deserializer, Serializer};

pub fn serialize<S>(dur: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(dur.num_seconds())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::seconds(i64::deserialize(deserializer)?))
}

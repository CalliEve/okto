use serde::{self, Deserialize, Deserializer, Serializer};

pub fn serialize<S>(string: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    Ok(match string {
        Some(ref s) => serializer.serialize_str(s)?,
        None => serializer.serialize_none()?,
    })
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let string = Option::<String>::deserialize(deserializer)?;

    Ok(match string {
        None => None,
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s),
    })
}

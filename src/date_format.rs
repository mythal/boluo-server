use chrono::{NaiveDateTime};
use serde::{self, Deserialize, Serializer, Deserializer};


pub fn serialize<S>(
    date: &NaiveDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
{
    serializer.serialize_i64(date.timestamp_millis())
}

pub fn deserialize<'de, D>(
    deserializer: D,
) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
{
    let timestamp = i64::deserialize(deserializer)?;
    let secs = timestamp / 1000;
    let millis = (timestamp % 1000) as u32;
    let nsecs = millis * 1_000_000;
    Ok(NaiveDateTime::from_timestamp(secs, nsecs))
}

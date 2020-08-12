use chrono::NaiveDateTime;
use serde::{self, Deserialize, Deserializer, Serializer};

pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(date.timestamp_millis())
}

pub fn timestamp_to_date_time(timestamp: i64) -> NaiveDateTime {
    let secs = timestamp / 1000;
    let millis = (timestamp % 1000) as u32;
    let nsecs = millis * 1_000_000;
    NaiveDateTime::from_timestamp(secs, nsecs)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp = i64::deserialize(deserializer)?;
    Ok(timestamp_to_date_time(timestamp))
}

pub mod option {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(date: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(date) = date {
            serializer.serialize_i64(date.timestamp_millis())
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = Option::<i64>::deserialize(deserializer)?;
        if let Some(timestamp) = timestamp {
            Ok(Some(super::timestamp_to_date_time(timestamp)))
        } else {
            Ok(None)
        }
    }
}

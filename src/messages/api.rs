use serde::Deserialize;
use serde_json::Value as JsonValue;
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewMessage {
    pub message_id: Option<Uuid>,
    pub channel_id: Uuid,
    pub name: String,
    pub text: String,
    pub entities: Vec<JsonValue>,
    pub in_game: bool,
    pub is_action: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewPreview {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub name: String,
    pub media_id: Option<Uuid>,
    pub in_game: bool,
    pub is_action: bool,
    pub text: String,
    pub entities: Vec<JsonValue>,
    pub whisper_to_users: Option<Vec<Uuid>>,
    #[serde(with = "crate::date_format")]
    pub start: NaiveDateTime,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub message_id: Uuid,
    pub name: Option<String>,
    pub text: Option<String>,
    pub entities: Option<Vec<JsonValue>>,
    pub in_game: Option<bool>,
    pub is_action: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ByChannel {
    pub channel_id: Uuid,
    pub before: Option<i64>,
    pub amount: Option<i32>,
}

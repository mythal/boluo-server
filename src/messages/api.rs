use serde::Deserialize;
use serde_json::Value as JsonValue;
use uuid::Uuid;

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
    pub order_date: Option<i64>,
    pub media_id: Option<Uuid>,
    pub whisper_to_users: Option<Vec<Uuid>>,
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
    pub media_id: Option<Uuid>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MoveToMode {
    Top,
    Bottom,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MoveTo {
    pub channel_id: Uuid,
    pub message_id: Uuid,
    pub target_id: Uuid,
    pub mode: MoveToMode,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MovePosBetween {
    pub message_id: Uuid,
    pub a: Option<f64>,
    pub b: Option<f64>,
    pub channel_id: Uuid,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ByChannel {
    pub channel_id: Uuid,
    pub before: Option<f64>,
    pub limit: Option<i32>,
}

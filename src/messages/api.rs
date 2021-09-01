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
    pub media_id: Option<Uuid>,
    pub whisper_to_users: Option<Vec<Uuid>>,
    pub pos: Option<f64>,
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
pub struct MoveBetween {
    pub message_id: Uuid,
    pub range: (Option<f64>, Option<f64>),
    pub channel_id: Uuid,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ByChannel {
    pub channel_id: Uuid,
    pub before: Option<f64>,
    pub limit: Option<i32>,
}

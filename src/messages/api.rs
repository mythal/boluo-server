use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewMessage {
    pub message_id: Option<Uuid>,
    pub channel_id: Uuid,
    pub name: Option<String>,
    pub text: String,
    pub entities: Value,
    pub in_game: bool,
    pub is_action: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub message_id: Uuid,
    pub name: Option<String>,
    pub text: Option<String>,
    pub entities: Option<Value>,
    pub in_game: Option<bool>,
    pub is_action: Option<bool>,
}

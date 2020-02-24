use serde::Deserialize;
use uuid::Uuid;
use crate::messages::api::NewPreview;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventQuery {
    pub mailbox: Uuid,
    pub after: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ClientEvent {
    Preview (NewPreview),
}

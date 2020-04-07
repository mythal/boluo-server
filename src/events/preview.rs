use crate::channels::ChannelMember;
use crate::database;
use crate::error::AppError;
use crate::events::Event;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub channel_id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub name: String,
    pub media_id: Option<Uuid>,
    pub in_game: bool,
    pub is_action: bool,
    pub is_master: bool,
    pub text: Option<String>,
    pub whisper_to_users: Option<Vec<Uuid>>,
    pub entities: Vec<JsonValue>,
    #[serde(with = "crate::date_format")]
    pub start: NaiveDateTime,
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
    pub text: Option<String>,
    pub entities: Vec<JsonValue>,
    #[serde(with = "crate::date_format")]
    pub start: NaiveDateTime,
}

impl NewPreview {
    pub async fn broadcast(self, user_id: Uuid) -> Result<(), AppError> {
        let NewPreview {
            id,
            channel_id,
            name,
            media_id,
            in_game,
            is_action,
            text,
            entities,
            start,
        } = self;

        let mut conn = database::get().await?;
        let db = &mut *conn;
        let member = ChannelMember::get(db, &user_id, &channel_id)
            .await?
            .ok_or(AppError::NoPermission)?;
        Event::message_preview(Preview {
            id,
            sender_id: user_id,
            channel_id,
            parent_message_id: None,
            name,
            media_id,
            in_game,
            is_action,
            text,
            whisper_to_users: None,
            entities,
            start,
            is_master: member.is_master,
        });
        Ok(())
    }
}

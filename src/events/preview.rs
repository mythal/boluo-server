use crate::channels::ChannelMember;
use crate::database;
use crate::error::AppError;
use crate::events::Event;
use crate::{cache, error::Find};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub clear: bool,
    pub text: Option<String>,
    pub whisper_to_users: Option<Vec<Uuid>>,
    pub entities: Vec<JsonValue>,
    pub start: f64,
    pub pos: f64,
    #[serde(with = "crate::date_format::option")]
    pub edit_for: Option<NaiveDateTime>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPost {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub name: String,
    pub media_id: Option<Uuid>,
    pub in_game: bool,
    pub is_action: bool,
    pub text: Option<String>,
    #[serde(default)]
    pub clear: bool,
    pub entities: Vec<JsonValue>,
    #[serde(default)]
    #[serde(with = "crate::date_format::option")]
    pub edit_for: Option<NaiveDateTime>,
}

impl PreviewPost {
    pub async fn broadcast(self, space_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let PreviewPost {
            id,
            channel_id,
            name,
            media_id,
            in_game,
            is_action,
            text,
            entities,
            edit_for,
            clear,
        } = self;
        let mut conn = database::get().await?;
        let mut cache = cache::conn().await;
        let cache = &mut cache;
        let db = &mut *conn;
        if text.is_none() {
            crate::pos::finished(cache, &id).await?;
        }
        let start: f64 = crate::pos::pos(db, cache, &channel_id, &id).await? as f64;
        let is_master = ChannelMember::get(db, &user_id, &channel_id)
            .await
            .or_no_permission()?
            .is_master;
        let whisper_to_users = None;
        let preview = Box::new(Preview {
            id,
            sender_id: user_id,
            channel_id,
            parent_message_id: None,
            name,
            media_id,
            in_game,
            is_action,
            text,
            whisper_to_users,
            entities,
            start,
            is_master,
            edit_for,
            clear,
            pos: start,
        });
        Event::message_preview(space_id, preview);
        Ok(())
    }
}

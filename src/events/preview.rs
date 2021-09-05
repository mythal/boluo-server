use crate::cache::make_key;
use crate::channels::{ChannelMember};
use crate::database;
use crate::error::{AppError, CacheError};
use crate::events::Event;
use crate::{cache, error::Find};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use redis::AsyncCommands;
use crate::database::Querist;

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
    pub fn start_key(id: Uuid) -> Vec<u8> {
        make_key(b"preview", &id, b"start")
    }

    pub async fn get_start(cache: &mut cache::Connection, message_id: &Uuid) -> Result<Option<i64>, CacheError> {
        let preview_start_key = make_key(b"preview", message_id, b"start");
        let preview_start: Option<i64> = cache.inner.get(&preview_start_key).await?;
        Ok(preview_start)
    }

    pub async fn channel_start<T: Querist>(db: &mut T, cache: &mut cache::Connection, channel_id: &Uuid, reset: bool) -> Result<i64, CacheError> {
        let channel_start_key = make_key(b"channel", channel_id, b"start");
        let channel_start: Option<i64> = cache.inner.get(&channel_start_key).await?;
        if channel_start.is_none() || reset {
            let initial_pos = crate::messages::Message::max_pos(db, channel_id).await.floor();
            let _: () = cache.inner.set(&channel_start_key, initial_pos as i64 + 1).await?;
        }
        let channel_start: i64 = cache.inner.incr(&channel_start_key, 1).await?;
        Ok(channel_start)
    }

    async fn start<T: Querist>(db: &mut T, channel_id: &Uuid, message_id: &Uuid, new: bool) -> Result<i64, CacheError> {
        let preview_start_key = make_key(b"preview", message_id, b"start");
        let mut cache = cache::conn().await;
        let preview_start: Option<i64> = cache.inner.get(&preview_start_key).await?;
        if let (false, Some(preview_start)) = (new, preview_start) {
            Ok(preview_start)
        } else {
            let channel_start: i64 = PreviewPost::channel_start(db, &mut cache, channel_id, false).await?;
            cache.inner.set_ex(&preview_start_key, channel_start, 60 * 5).await?;
            Ok(channel_start)
        }
    }

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
        let db = &mut *conn;
        let start: f64 = PreviewPost::start(db, &channel_id, &id, text.is_none()).await? as f64;
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

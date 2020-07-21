use crate::channels::ChannelMember;
use crate::database;
use crate::cache;
use crate::error::AppError;
use crate::events::Event;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use crate::cache::make_key;
use crate::events::events::MailBoxType;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub mailbox: Uuid,
    pub mailbox_type: MailBoxType,
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
pub struct PreviewPost {
    pub id: Uuid,
    pub name: String,
    pub media_id: Option<Uuid>,
    pub in_game: bool,
    pub is_action: bool,
    pub text: Option<String>,
    pub entities: Vec<JsonValue>,
}

impl PreviewPost {
    pub async fn broadcast(self, mailbox: Uuid, mailbox_type: MailBoxType, user_id: Uuid) -> Result<(), AppError> {
        let PreviewPost {
            id,
            name,
            media_id,
            in_game,
            is_action,
            text,
            entities,
        } = self;
        let start = {
            let mut cache = cache::conn().await;
            let key = make_key(b"preview", &id, b"start");
            if let Some(bytes) = cache.get(&key).await? {
                serde_json::from_slice(&*bytes).map_err(error_unexpected!())?
            } else {
                let now = chrono::Local::now().naive_utc();
                let bytes = serde_json::to_vec(&now).map_err(error_unexpected!())?;
                cache.set_with_expiration(&key, &*bytes, 60 * 5).await?;
                now
            }
        };
        let mut conn = database::get().await?;
        let db = &mut *conn;
        let is_master = match mailbox_type {
            MailBoxType::Channel => {
                ChannelMember::get(db, &user_id, &mailbox)
                    .await?
                    .ok_or(AppError::NoPermission)?.is_master
            }
        };
        Event::message_preview(Preview {
            id,
            sender_id: user_id,
            mailbox,
            mailbox_type,
            parent_message_id: None,
            name,
            media_id,
            in_game,
            is_action,
            text,
            whisper_to_users: None,
            entities,
            start,
            is_master,
        });
        Ok(())
    }
}

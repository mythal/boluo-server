use crate::channels::models::Member;
use crate::channels::Channel;

use crate::error::log_error;
use crate::events::context;
use crate::events::context::{get_heartbeat_map, SyncEvent};
use crate::events::preview::{Preview, PreviewPost};
use crate::messages::{Message, MessageOrder};
use crate::spaces::api::SpaceWithRelated;
use crate::spaces::models::{StatusKind, UserStatus, space_users_status};
use crate::utils::timestamp;
use crate::{cache, database};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::spawn;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventQuery {
    pub mailbox: Uuid,
}


#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type")]
pub enum ClientEvent {
    #[serde(rename_all = "camelCase")]
    Preview { preview: PreviewPost },
    #[serde(rename_all = "camelCase")]
    Status { kind: StatusKind, focus: Vec<Uuid>, },
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventBody {
    #[serde(rename_all = "camelCase")]
    NewMessage {
        message: Box<Message>,
    },
    #[serde(rename_all = "camelCase")]
    MessageDeleted {
        message_id: Uuid,
        channel_id: Uuid,
    },
    #[serde(rename_all = "camelCase")]
    MessagesMoved {
        moved_messages: Vec<Message>,
        order_changes: Vec<MessageOrder>,
    },
    #[serde(rename_all = "camelCase")]
    MessageEdited {
        message: Box<Message>,
    },
    #[serde(rename_all = "camelCase")]
    MessagePreview {
        preview: Box<Preview>,
    },
    ChannelDeleted { channel_id: Uuid },
    #[serde(rename_all = "camelCase")]
    ChannelEdited {
        channel: Channel,
    },
    #[serde(rename_all = "camelCase")]
    Members {
        channel_id: Uuid,
        members: Vec<Member>,
    },
    Initialized,
    StatusMap {
        status_map: HashMap<Uuid, UserStatus>,
    },
    #[serde(rename_all = "camelCase")]
    SpaceUpdated {
        space_with_related: SpaceWithRelated,
    },
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub mailbox: Uuid,
    pub timestamp: i64,
    pub body: EventBody,
}

impl Event {
    pub fn initialized(mailbox: Uuid) -> Event {
        Event {
            mailbox,
            timestamp: timestamp(),
            body: EventBody::Initialized,
        }
    }

    pub fn new_message(space_id: Uuid, message: Message) {
        let message = Box::new(message);
        Event::fire(EventBody::NewMessage { message }, space_id)
    }

    pub fn message_deleted(space_id: Uuid, channel_id: Uuid, message_id: Uuid) {
        Event::fire(
            EventBody::MessageDeleted { message_id, channel_id },
            space_id,
        )
    }

    pub fn message_edited(space_id: Uuid, message: Message) {
        let message = Box::new(message);
        Event::fire(EventBody::MessageEdited { message }, space_id)
    }
    pub fn messages_moved(space_id: Uuid, moved_messages: Vec<Message>, order_changes: Vec<MessageOrder>) {
        if moved_messages.is_empty() && order_changes.is_empty() {
            return;
        }
        Event::fire(
            EventBody::MessagesMoved {
                moved_messages,
                order_changes,
            },
            space_id,
        )
    }

    pub fn channel_deleted(space_id: Uuid, channel_id: Uuid) {
        Event::fire(EventBody::ChannelDeleted { channel_id }, space_id)
    }

    pub fn message_preview(space_id: Uuid, preview: Box<Preview>) {
        Event::fire(EventBody::MessagePreview { preview }, space_id);
    }

    pub async fn heartbeat(mailbox: Uuid, user_id: Uuid) {
        let now = timestamp();
        let map = get_heartbeat_map();
        let mut map = map.lock().await;
        if let Some(heartbeat_map) = map.get_mut(&mailbox) {
            heartbeat_map.insert(user_id, now);
        } else {
            let mut heartbeat_map = HashMap::new();
            heartbeat_map.remove(&user_id);
            heartbeat_map.insert(user_id, now);
            map.insert(mailbox, heartbeat_map);
        }
    }

    pub async fn status(space_id: Uuid, user_id: Uuid, kind: StatusKind, timestamp: i64, focus: Vec<Uuid>) {
        let cache = cache::conn().await;
        let mut redis = cache.inner;
        let heartbeat = UserStatus { timestamp, kind, focus };
    
        let key = cache::make_key(b"space", &space_id, b"heartbeat");
        let value = serde_json::to_vec(&heartbeat).unwrap();
    
        if let Err(err) = redis.hset::<_, _, _, bool>(&*key, user_id.as_bytes(), &*value).await {
            log::error!("failed to set user state: {}", err);
        }
        space_users_status(&mut redis, space_id).await.ok();
    }

    pub fn push_members(channel_id: Uuid) {
        spawn(async move {
            if let Err(e) = Event::fire_members(channel_id).await {
                log::warn!("Failed to fetch member list: {}", e);
            }
        });
    }

    pub fn channel_edited(channel: Channel) {
        let space_id = channel.space_id;
        Event::fire(EventBody::ChannelEdited { channel }, space_id);
    }

    pub fn cache_key(mailbox: &Uuid) -> Vec<u8> {
        cache::make_key(b"mailbox", mailbox, b"events")
    }

    pub async fn get_from_cache(mailbox: &Uuid) -> Vec<String> {
        let cache = super::context::get_cache().try_mailbox(mailbox).await;
        if let Some(cache) = cache {
            let cache = cache.lock().await;
            cache
                .edition_map
                .values()
                .chain(cache.preview_map.values())
                .chain(cache.events.iter())
                .map(|event| event.encoded.clone())
                .collect()
        } else {
            vec![]
        }
    }

    pub fn space_updated(space_id: Uuid) {
        tokio::spawn(async move {
            match crate::spaces::handlers::space_related(&space_id).await {
                Ok(space_with_related) => {
                    Event::async_fire(
                        EventBody::SpaceUpdated { space_with_related },
                        space_id,
                    )
                    .await
                }
                Err(e) => log_error(&e, "event"),
            }
        });
    }

    async fn send(mailbox: Uuid, event: Arc<SyncEvent>) {
        let broadcast_table = context::get_broadcast_table();
        let table = broadcast_table.read().await;
        if let Some(tx) = table.get(&mailbox) {
            tx.send(event).ok();
        }
    }

    async fn fire_members(channel_id: Uuid) -> Result<(), anyhow::Error> {
        let mut db = database::get().await?;
        let members = Member::get_by_channel(&mut *db, channel_id).await?;
        drop(db);
        let event = SyncEvent::new(Event {
            mailbox: channel_id,
            body: EventBody::Members { members, channel_id },
            timestamp: timestamp(),
        });

        Event::send(channel_id, Arc::new(event)).await;
        Ok(())
    }

    fn build(body: EventBody, mailbox: Uuid) -> Arc<SyncEvent> {
        Arc::new(SyncEvent::new(Event {
            mailbox,
            body,
            timestamp: timestamp(),
        }))
    }

    async fn async_fire(body: EventBody, mailbox: Uuid) {
        let cache = super::context::get_cache().mailbox(&mailbox).await;
        let mut cache = cache.lock().await;

        enum Kind {
            Preview(Uuid),
            Edition(Uuid),
            Other,
        }

        let kind = match &body {
            EventBody::MessagePreview { preview } => {
                if preview.edit_for.is_some() {
                    Kind::Edition(preview.id)
                } else {
                    Kind::Preview(preview.sender_id)
                }
            }
            _ => Kind::Other,
        };

        let event = Event::build(body, mailbox);
        match kind {
            Kind::Edition(id) => {
                cache.edition_map.insert(id, event.clone());
            }
            Kind::Preview(id) => {
                cache.preview_map.insert(id, event.clone());
            }
            Kind::Other => {
                cache.events.push_back(event.clone());
            }
        }

        Event::send(mailbox, event).await;
    }

    pub fn fire(body: EventBody, mailbox: Uuid) {
        spawn(Event::async_fire(body, mailbox));
    }
}

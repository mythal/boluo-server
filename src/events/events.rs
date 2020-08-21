use crate::channels::models::Member;
use crate::channels::Channel;

use crate::events::context;
use crate::events::context::{get_heartbeat_map, SyncEvent};
use crate::events::preview::{Preview, PreviewPost};
use crate::messages::{Message, MessageOrder};
use crate::utils::timestamp;
use crate::{cache, database};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::spawn;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventQuery {
    pub mailbox: Uuid,
    pub mailbox_type: MailBoxType,
    /// timestamp
    pub after: i64,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailBoxType {
    Channel,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type")]
pub enum ClientEvent {
    #[serde(rename_all = "camelCase")]
    Preview { preview: PreviewPost },
    #[serde(rename_all = "camelCase")]
    Heartbeat,
}

#[derive(Serialize, Debug, Clone)]
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
    ChannelDeleted,
    #[serde(rename_all = "camelCase")]
    ChannelEdited {
        channel: Channel,
    },
    #[serde(rename_all = "camelCase")]
    Members {
        members: Vec<Member>,
    },
    Initialized,
    #[serde(rename_all = "camelCase")]
    Heartbeat {
        user_id: Uuid,
    },
    #[serde(rename_all = "camelCase")]
    HeartbeatMap {
        heartbeat_map: HashMap<Uuid, i64>,
    },
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub mailbox: Uuid,
    pub mailbox_type: MailBoxType,
    pub timestamp: i64,
    pub body: EventBody,
}

impl Event {
    pub fn initialized(mailbox: Uuid, mailbox_type: MailBoxType) -> Event {
        Event {
            mailbox,
            mailbox_type,
            timestamp: timestamp(),
            body: EventBody::Initialized,
        }
    }

    pub async fn push_heartbeat_map(channel_id: Uuid, heartbeat_map: HashMap<Uuid, i64>) {
        let event = SyncEvent::new(Event {
            mailbox: channel_id,
            mailbox_type: MailBoxType::Channel,
            timestamp: timestamp(),
            body: EventBody::HeartbeatMap { heartbeat_map },
        });
        Event::send(channel_id, Arc::new(event)).await;
    }

    pub fn new_message(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        Event::fire(EventBody::NewMessage { message }, channel_id, MailBoxType::Channel)
    }

    pub fn message_deleted(channel_id: Uuid, message_id: Uuid) {
        Event::fire(
            EventBody::MessageDeleted { message_id },
            channel_id,
            MailBoxType::Channel,
        )
    }

    pub fn message_edited(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        Event::fire(EventBody::MessageEdited { message }, channel_id, MailBoxType::Channel)
    }
    pub fn messages_moved(channel_id: Uuid, moved_messages: Vec<Message>, order_changes: Vec<MessageOrder>) {
        if moved_messages.is_empty() && order_changes.is_empty() {
            return;
        }
        Event::fire(
            EventBody::MessagesMoved {
                moved_messages,
                order_changes,
            },
            channel_id,
            MailBoxType::Channel,
        )
    }

    pub fn channel_deleted(channel_id: Uuid) {
        Event::fire(EventBody::ChannelDeleted, channel_id, MailBoxType::Channel)
    }

    pub fn message_preview(preview: Box<Preview>) {
        let mailbox = preview.mailbox;
        let mailbox_type = preview.mailbox_type;
        Event::fire(EventBody::MessagePreview { preview }, mailbox, mailbox_type);
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

    pub fn push_members(channel_id: Uuid) {
        spawn(async move {
            if let Err(e) = Event::fire_members(channel_id).await {
                log::warn!("Failed to fetch member list: {}", e);
            }
        });
    }

    pub fn channel_edited(channel: Channel) {
        let channel_id = channel.id;
        Event::fire(EventBody::ChannelEdited { channel }, channel_id, MailBoxType::Channel);
    }

    pub fn cache_key(mailbox: &Uuid) -> Vec<u8> {
        cache::make_key(b"mailbox", mailbox, b"events")
    }

    pub async fn get_from_cache(mailbox: &Uuid, after: i64) -> Vec<String> {
        let cache = super::context::get_cache().try_channel(mailbox).await;
        if let Some(cache) = cache {
            let cache = cache.lock().await;
            let events = cache
                .events
                .iter()
                .skip_while(|event| event.event.timestamp <= after);
            cache
                .edition_map
                .values()
                .chain(cache.preview_map.values())
                .filter(|event| event.event.timestamp > after)
                .chain(events)
                .map(|event| event.encoded.clone())
                .collect()
        } else {
            vec![]
        }
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
            mailbox_type: MailBoxType::Channel,
            body: EventBody::Members { members },
            timestamp: timestamp(),
        });

        Event::send(channel_id, Arc::new(event)).await;
        Ok(())
    }

    fn build(body: EventBody, mailbox: Uuid, mailbox_type: MailBoxType) -> Arc<SyncEvent> {
        Arc::new(SyncEvent::new(Event {
            mailbox,
            body,
            mailbox_type,
            timestamp: timestamp(),
        }))
    }

    async fn async_fire(body: EventBody, mailbox: Uuid, mailbox_type: MailBoxType) {
        let cache = super::context::get_cache().channel(&mailbox).await;
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
            },
            _ => {
                Kind::Other
            },
        };

        let event = Event::build(body, mailbox, mailbox_type);
        match kind {
            Kind::Edition(id) => { cache.edition_map.insert(id, event.clone()); },
            Kind::Preview(id) => { cache.preview_map.insert(id, event.clone()); },
            Kind::Other => { cache.events.push_back(event.clone()); },
        }

        Event::send(mailbox, event).await;
    }

    pub fn fire(body: EventBody, mailbox: Uuid, mailbox_type: MailBoxType) {
        spawn(Event::async_fire(body, mailbox, mailbox_type));
    }
}

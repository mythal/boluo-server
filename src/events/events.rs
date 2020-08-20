use crate::channels::models::Member;
use crate::channels::Channel;

use crate::events::context;
use crate::events::context::{get_heartbeat_map, SyncEvent};
use crate::events::preview::{Preview, PreviewPost};
use crate::messages::Message;
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
        messages: Vec<Message>,
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
    pub fn messages_removed(messages: Vec<Message>) {
        if messages.len() == 0 {
            return;
        }
        let channel_id = messages[0].channel_id;
        Event::fire(EventBody::MessagesMoved { messages }, channel_id, MailBoxType::Channel)
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
                .skip_while(|event| event.event.timestamp < after)
                .map(|event| event.encoded.clone())
                .collect();
            events
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

    async fn async_fire(body: EventBody, mailbox: Uuid, mailbox_type: MailBoxType) {
        let preview_info = match &body {
            EventBody::MessagePreview { preview } => Some((preview.id, preview.sender_id, preview.edit_for)),
            _ => None,
        };
        let event = Arc::new(SyncEvent::new(Event {
            mailbox,
            body,
            mailbox_type,
            timestamp: timestamp(),
        }));
        let cache = super::context::get_cache().channel(&mailbox).await;
        let mut cache = cache.lock().await;

        let events = &mut cache.events;
        if let Some((preview_id, sender_id, edit_for)) = preview_info {
            if let Some((i, _)) = events
                .iter()
                .rev()
                .enumerate()
                .take(16)
                .find(|(_, e)| match &e.event.body {
                    EventBody::MessagePreview { preview } => {
                        preview.sender_id == sender_id
                            && (preview.id == preview_id || edit_for.is_none())
                            && preview.edit_for == edit_for
                    }
                    _ => false,
                })
            {
                let index = events.len() - 1 - i;
                events[index] = event.clone();
            } else {
                events.push_back(event.clone());
            }
        } else {
            events.push_back(event.clone());
        }

        Event::send(mailbox, event).await;
    }

    pub fn fire(body: EventBody, mailbox: Uuid, mailbox_type: MailBoxType) {
        spawn(Event::async_fire(body, mailbox, mailbox_type));
    }
}

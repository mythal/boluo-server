use crate::cache;
use crate::cache::make_key;
use crate::error::CacheError;
use crate::messages::{Message, Preview};
use crate::utils::{self, timestamp};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

mod api;
mod handlers;
mod models;

pub use handlers::router;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_id: Uuid,
    pub mailbox: Uuid,
    pub timestamp: i64,
    pub body: EventBody,
}

impl Event {
    pub fn new(mailbox: Uuid, body: EventBody) -> Event {
        Event {
            event_id: utils::id(),
            timestamp: timestamp(),
            mailbox,
            body,
        }
    }

    pub fn new_message(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        let event = Event::new(channel_id, EventBody::NewMessage { message });
        Event::fire(event, channel_id)
    }

    pub fn message_deleted(channel_id: Uuid, message_id: Uuid) {
        let event = Event::new(channel_id, EventBody::MessageDeleted { message_id });
        Event::fire(event, channel_id)
    }

    pub fn message_edited(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        let event = Event::new(channel_id, EventBody::MessageEdited { message });
        Event::fire(event, channel_id)
    }

    pub fn channel_deleted(channel_id: Uuid) {
        let event = Event::new(channel_id, EventBody::ChannelDeleted);
        Event::fire(event, channel_id)
    }

    pub fn message_preview(preview: Preview) {
        let channel_id = preview.channel_id;
        let preview = Box::new(preview);
        let event = Event::new(channel_id, EventBody::MessagePreview { preview });
        Event::fire(event, channel_id)
    }

    pub fn cache_key(mailbox: &Uuid) -> Vec<u8> {
        make_key(b"mailbox", mailbox, b"events")
    }

    pub async fn get_from_cache(mailbox: &Uuid, after: i64) -> Result<Vec<Event>, CacheError> {
        let mut cache = cache::get().await;
        let bytes_array = cache.get_after(&*Self::cache_key(mailbox), after + 1).await?;
        let mut events = Vec::with_capacity(bytes_array.len());
        for bytes in bytes_array.into_iter() {
            let event: Result<Event, _> = serde_json::from_slice(&*bytes);
            match event {
                Err(e) => {
                    log::debug!("{:?}", bytes);
                    log::error!("Failed to deserialize event: {}", e)
                },
                Ok(e) => events.push(e),
            }
        }
        Ok(events)
    }

    pub async fn wait(mailbox: Uuid) -> Result<Arc<Event>, tokio::sync::broadcast::RecvError> {
        get_receiver(&mailbox).await.recv().await
    }

    pub fn fire(event: Event, mailbox: Uuid) {
        use tokio::spawn;
        let event = Arc::new(event);
        spawn(async move {
            let broadcast_table = get_broadcast_table();
            let table = broadcast_table.read().await;
            if let Some(tx) = table.get(&mailbox) {
                tx.send(event.clone()).ok();
            }
            drop(table);
            let mut cache = cache::get().await;
            match serde_json::to_vec(&*event) {
                Ok(encoded) => {
                    let key = Self::cache_key(&mailbox);
                    if let Err(e) = cache.set_with_time(&*key, &*encoded).await {
                        log::warn!("Failed to add event to redis: {}", e)
                    }
                }
                Err(e) => {
                    log::error!("Failed to serialize event: {}", e);
                }
            }
            drop(cache);
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum EventBody {
    NewMessage { message: Box<Message> },
    MessageDeleted { message_id: Uuid },
    MessageEdited { message: Box<Message> },
    MessagePreview { preview: Box<Preview> },
    ChannelDeleted,
    ChannelEdited,
    Refresh,
}

type BroadcastTable = RwLock<HashMap<Uuid, broadcast::Sender<Arc<Event>>>>;

static BROADCAST_TABLE: OnceCell<BroadcastTable> = OnceCell::new();

fn get_broadcast_table() -> &'static BroadcastTable {
    BROADCAST_TABLE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn get_receiver(id: &Uuid) -> broadcast::Receiver<Arc<Event>> {
    let broadcast_table = get_broadcast_table();
    let table = broadcast_table.read().await;
    if let Some(sender) = table.get(id) {
        sender.subscribe()
    } else {
        drop(table);
        let capacity = 16;
        let (tx, rx) = broadcast::channel(capacity);
        let mut table = broadcast_table.write().await;
        table.insert(id.clone(), tx);
        rx
    }
}

async fn redis_cleaner() {
    use redis::{AsyncCommands, RedisError};
    use std::time::Duration;
    use tokio::time::delay_for;
    loop {
        delay_for(Duration::from_secs(12 * 60 * 60)).await;
        let mut cache = cache::get().await;
        let keys: Result<Vec<Vec<u8>>, RedisError> = cache.inner.keys(b"mailbox:*").await;
        match keys {
            Ok(keys) => {
                let before = timestamp() - 24 * 60 * 60 * 1000;
                for key in keys.into_iter() {
                    if let Err(e) = cache.clear_before(&*key, before).await {
                        log::warn!("Failed to clear old events: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get keys of events from Redis: {}", e);
            }
        }
        log::info!("Redis clean finished");
    }
}

pub async fn periodical_cleaner() {
    use std::time::Duration;
    use tokio::time::delay_for;
    tokio::spawn(redis_cleaner());
    loop {
        delay_for(Duration::from_secs(15)).await;
        let mut broadcast_table = get_broadcast_table().write().await;
        broadcast_table.retain(|_, v| v.receiver_count() != 0);
        drop(broadcast_table);
        log::trace!("clean finished");
    }
}

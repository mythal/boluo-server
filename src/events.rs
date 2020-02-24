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
use futures::StreamExt;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_id: Uuid,
    pub mailbox: Uuid,
    pub timestamp: i64,
    pub body: EventBody,
}

impl Event {
    pub fn new_message(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        Event::fire(EventBody::NewMessage { message }, channel_id)
    }

    pub fn message_deleted(channel_id: Uuid, message_id: Uuid) {
        Event::fire(EventBody::MessageDeleted { message_id }, channel_id)
    }

    pub fn message_edited(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        Event::fire(EventBody::MessageEdited { message }, channel_id)
    }

    pub fn channel_deleted(channel_id: Uuid) {
        Event::fire(EventBody::ChannelDeleted, channel_id)
    }

    pub fn message_preview(preview: Preview) {
        let channel_id = preview.channel_id;
        let preview = Box::new(preview);
        Event::fire(EventBody::MessagePreview { preview }, channel_id)
    }

    pub fn cache_key(mailbox: &Uuid) -> Vec<u8> {
        make_key(b"mailbox", mailbox, b"events")
    }

    pub async fn get_from_cache(mailbox: &Uuid, after: i64) -> Result<Vec<String>, CacheError> {
        let mut cache = cache::get().await;
        let bytes_array = cache.get_after(&*Self::cache_key(mailbox), after + 1).await?;
        let events = bytes_array
            .into_iter()
            .map(|bytes| String::from_utf8(bytes).ok())
            .filter_map(|s| s)
            .collect();
        Ok(events)
    }

    pub async fn wait(mailbox: Uuid) -> Result<String, tokio::sync::broadcast::RecvError> {
        get_receiver(&mailbox).await.recv().await
    }

    async fn async_fire(body: EventBody, mailbox: Uuid) -> Result<(), anyhow::Error> {
        let mut cache = cache::get().await;
        let event = Arc::new(Event{
            mailbox,
            body,
            timestamp: timestamp(),
            event_id: utils::id(),
        });
        let encoded = serde_json::to_string(&*event)?;
        let key = Self::cache_key(&mailbox);
        cache.set_with_time(&*key, encoded.as_bytes()).await?;
        drop(cache);
        let broadcast_table = get_broadcast_table();
        let table = broadcast_table.read().await;
        if let Some(tx) = table.get(&mailbox) {
            tx.send(encoded).ok();
        }
        Ok(())
    }

    pub fn fire(body: EventBody, mailbox: Uuid) {
        use tokio::spawn;
        spawn(async move {
            if let Err(e) = Event::async_fire(body, mailbox).await {
                log::warn!("Error on fire event: {}", e);
            }
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

type BroadcastTable = RwLock<HashMap<Uuid, broadcast::Sender<String>>>;

static BROADCAST_TABLE: OnceCell<BroadcastTable> = OnceCell::new();

fn get_broadcast_table() -> &'static BroadcastTable {
    BROADCAST_TABLE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn get_receiver(id: &Uuid) -> broadcast::Receiver<String> {
    let broadcast_table = get_broadcast_table();
    let table = broadcast_table.read().await;
    if let Some(sender) = table.get(id) {
        sender.subscribe()
    } else {
        drop(table);
        let capacity = 256;
        let (tx, rx) = broadcast::channel(capacity);
        let mut table = broadcast_table.write().await;
        table.insert(id.clone(), tx);
        rx
    }
}

pub async fn periodical_cleaner() {
    use redis::{AsyncCommands, RedisError};
    use std::time::Duration;
    use tokio::time::interval;
    let broadcast_clean = interval(Duration::from_secs(5 * 60))
        .for_each(|_| async {
            let mut broadcast_table = get_broadcast_table().write().await;
            broadcast_table.retain(|_, v| v.receiver_count() != 0);
            drop(broadcast_table);
            log::trace!("clean finished");
        });
    let redis_clean = interval(Duration::from_secs(12 * 60 * 60))
        .for_each(|_| async {
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
        });
    futures::pin_mut!(broadcast_clean);
    futures::pin_mut!(redis_clean);
    futures::future::select(broadcast_clean, redis_clean).await;
}

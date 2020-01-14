use crate::cache;
use crate::cache::make_key;
use crate::error::CacheError;
use crate::messages::{Message, Preview};
use crate::utils::{self, timestamp};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

mod api;
mod handlers;
mod models;

struct EventQueue {
    queue: VecDeque<Arc<Event>>,
    last_timestamp: i64,
}

const LOCAL_EVENTS_CACHE_TIME_MS: i64 = 60 * 1000;
static EVENT_QUEUE: OnceCell<RwLock<EventQueue>> = OnceCell::new();

impl EventQueue {
    fn new() -> EventQueue {
        EventQueue {
            queue: VecDeque::new(),
            last_timestamp: timestamp(),
        }
    }

    pub fn get() -> &'static RwLock<EventQueue> {
        EVENT_QUEUE.get_or_init(|| RwLock::new(EventQueue::new()))
    }

    fn clear_old(&mut self) {
        let now = timestamp();
        while self.last_timestamp < now - LOCAL_EVENTS_CACHE_TIME_MS {
            if let Some(event) = self.queue.pop_front() {
                self.last_timestamp = event.timestamp;
            } else {
                self.last_timestamp = now;
                break;
            }
        }
    }

    fn push(&mut self, event: Arc<Event>) {
        self.queue.push_back(event);
    }

    pub async fn get_events(since: i64, mailbox: &Uuid) -> Vec<Arc<Event>> {
        EventQueue::get()
            .read()
            .await
            .queue
            .iter()
            .rev()
            .take_while(|e| e.timestamp > since)
            .filter(|e| e.mailbox == *mailbox)
            .map(Clone::clone)
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
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
        let event = Arc::new(Event::new(channel_id, EventBody::NewMessage { message }));
        Event::fire(event, channel_id)
    }

    pub fn message_deleted(channel_id: Uuid, message_id: Uuid) {
        let event = Arc::new(Event::new(channel_id, EventBody::MessageDeleted { message_id }));
        Event::fire(event, channel_id)
    }

    pub fn message_edited(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        let event = Arc::new(Event::new(channel_id, EventBody::MessageEdited { message }));
        Event::fire(event, channel_id)
    }

    pub fn channel_deleted(channel_id: Uuid) {
        let event = Arc::new(Event::new(channel_id, EventBody::ChannelDeleted));
        Event::fire(event, channel_id)
    }

    pub fn message_preview(preview: Preview) {
        let channel_id = preview.channel_id;
        let preview = Box::new(preview);
        let event = Arc::new(Event::new(channel_id, EventBody::MessagePreview { preview }));
        Event::fire(event, channel_id)
    }

    pub fn cache_key(mailbox: &Uuid) -> Vec<u8> {
        make_key(b"mailbox", mailbox, b"events")
    }

    pub async fn get_from_cache(mailbox: &Uuid, since: i64) -> Result<Vec<Event>, CacheError> {
        let mut cache = cache::get().await;
        let bytes_array = cache.get_after(&*Self::cache_key(mailbox), since).await?;
        let mut events = Vec::with_capacity(bytes_array.len());
        for bytes in bytes_array.into_iter() {
            let event: Result<Event, _> = bincode::deserialize(&*bytes);
            match event {
                Err(e) => log::error!("Failed to deserialize event: {}", e),
                Ok(e) => events.push(e),
            }
        }
        Ok(events)
    }

    pub async fn wait(mailbox: Uuid) {
        use tokio::sync::broadcast::RecvError;

        match get_receiver(&mailbox).await.recv().await {
            Ok(()) | Err(RecvError::Lagged(_)) => (),
            Err(RecvError::Closed) => log::warn!("The subscription channel ({}) was close", mailbox),
        }
    }

    pub fn fire(event: Arc<Event>, mailbox: Uuid) {
        use tokio::spawn;
        let event_copy = event.clone();
        spawn(async move {
            let event = event_copy;
            let mut queue = EventQueue::get().write().await;
            queue.push(event);
            drop(queue);
            let broadcast_table = get_broadcast_table();
            let table = broadcast_table.read().await;
            if let Some(tx) = table.get(&mailbox) {
                tx.send(()).ok();
            }
            drop(table);
        });

        spawn(async move {
            let mut cache = cache::get().await;
            match bincode::serialize(&event) {
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
}

type BroadcastTable = RwLock<HashMap<Uuid, broadcast::Sender<()>>>;

static BROADCAST_TABLE: OnceCell<BroadcastTable> = OnceCell::new();

fn get_broadcast_table() -> &'static BroadcastTable {
    BROADCAST_TABLE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn get_receiver(id: &Uuid) -> broadcast::Receiver<()> {
    let broadcast_table = get_broadcast_table();
    let table = broadcast_table.read().await;
    if let Some(sender) = table.get(id) {
        sender.subscribe()
    } else {
        drop(table);
        let (tx, rx) = broadcast::channel(1);
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
        {
            let mut event_queue = EventQueue::get().write().await;
            event_queue.clear_old();
        }
        {
            let mut broadcast_table = get_broadcast_table().write().await;
            broadcast_table.retain(|_, v| v.receiver_count() != 0);
        }
        log::trace!("clean finished");
    }
}

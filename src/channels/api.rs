use super::models::{Channel, ChannelMember};
use crate::messages::{Message, Preview};
use crate::spaces::Space;
use crate::utils::timestamp;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    pub space_id: Uuid,
    pub name: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChannelWithRelated {
    pub channel: Channel,
    pub members: Vec<ChannelMember>,
    pub space: Space,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub struct Event {
    pub channel_id: Uuid,
    pub timestamp: i64,
    pub payload: EventPayload,
}

pub struct EventQueue {
    queue: VecDeque<Arc<Event>>,
    last_timestamp: i64,
}

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

    fn remove_old(&mut self) {
        let now = timestamp();
        let delta = 60000; // 60s
        while self.last_timestamp < now - delta {
            if let Some(event) = self.queue.pop_front() {
                self.last_timestamp = event.timestamp;
            } else {
                self.last_timestamp = now;
                break;
            }
        }
    }

    fn push(&mut self, event: Arc<Event>) {
        self.remove_old();
        self.queue.push_back(event);
    }

    pub fn get_events(&self, since: i64, channel_id: &Uuid) -> Vec<Arc<Event>> {
        if since < self.last_timestamp {
            let event = Event::need_refresh(channel_id.clone());
            return vec![Arc::new(event)];
        }
        self.queue
            .iter()
            .rev()
            .take_while(|e| e.timestamp > since)
            .filter(|e| e.channel_id == *channel_id)
            .map(Clone::clone)
            .collect()
    }
}

static EVENT_QUEUE: OnceCell<RwLock<EventQueue>> = OnceCell::new();

impl Event {
    pub fn new_message(message: Message) -> Event {
        Event {
            channel_id: message.channel_id.clone(),
            timestamp: timestamp(),
            payload: EventPayload::NewMessage { message },
        }
    }

    pub fn message_deleted(channel_id: Uuid, message_id: Uuid) -> Event {
        Event {
            channel_id: channel_id.clone(),
            timestamp: timestamp(),
            payload: EventPayload::MessageDeleted { channel_id, message_id },
        }
    }

    pub fn message_edited(channel_id: Uuid, message: Message) -> Event {
        Event {
            channel_id,
            timestamp: timestamp(),
            payload: EventPayload::MessageEdited { message },
        }
    }

    pub fn channel_deleted(channel_id: Uuid) -> Event {
        Event {
            channel_id: channel_id.clone(),
            timestamp: timestamp(),
            payload: EventPayload::ChannelDeleted { channel_id },
        }
    }

    pub fn need_refresh(channel_id: Uuid) -> Event {
        Event {
            channel_id,
            timestamp: timestamp(),
            payload: EventPayload::NeedRefresh,
        }
    }

    pub fn message_preview(channel_id: Uuid, preview: Preview) -> Event {
        Event {
            channel_id,
            timestamp: timestamp(),
            payload: EventPayload::MessagePreview { preview },
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum EventPayload {
    NewMessage { message: Message },
    MessageDeleted { channel_id: Uuid, message_id: Uuid },
    MessageEdited { message: Message },
    MessagePreview { preview: Preview },
    ChannelDeleted { channel_id: Uuid },
    NeedRefresh,
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

pub fn fire(channel_id: &Uuid, event: Event) {
    use tokio::spawn;
    let event = Arc::new(event);
    let channel_id = channel_id.clone();
    spawn(async move {
        let mut queue = EventQueue::get().write().await;
        queue.push(event);
        drop(queue);
        let broadcast_table = get_broadcast_table();
        let table = broadcast_table.read().await;
        if let Some(tx) = table.get(&channel_id) {
            if let Err(_) = tx.send(()) {
                drop(table);
                let mut table = broadcast_table.write().await;
                log::debug!("Event fired but no receiver.");
                table.remove(&channel_id);
            }
        }
    });
}

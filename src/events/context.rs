use crate::events::Event;
use once_cell::sync::OnceCell;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct SyncEvent {
    pub event: Event,
    pub encoded: String,
}

impl SyncEvent {
    pub fn new(event: Event) -> SyncEvent {
        let encoded = serde_json::to_string(&event).unwrap();
        SyncEvent { encoded, event }
    }
}

type BroadcastTable = RwLock<HashMap<Uuid, broadcast::Sender<Arc<SyncEvent>>>>;

static BROADCAST_TABLE: OnceCell<BroadcastTable> = OnceCell::new();

pub fn get_broadcast_table() -> &'static BroadcastTable {
    BROADCAST_TABLE.get_or_init(|| RwLock::new(HashMap::new()))
}

type HeartbeatMap = Mutex<HashMap<Uuid, HashMap<Uuid, i64>>>;
static HEARTBEAT_MAP: OnceCell<HeartbeatMap> = OnceCell::new();

pub fn get_heartbeat_map() -> &'static HeartbeatMap {
    HEARTBEAT_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub async fn get_mailbox_broadcast_rx(id: &Uuid) -> broadcast::Receiver<Arc<SyncEvent>> {
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

pub type EventMap = RwLock<HashMap<Uuid, VecDeque<Arc<SyncEvent>>>>;

static EVENT_MAP: OnceCell<EventMap> = OnceCell::new();

pub fn get_event_map() -> &'static EventMap {
    EVENT_MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

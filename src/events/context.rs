use tokio::sync::{broadcast, RwLock, Mutex};
use uuid::Uuid;
use std::collections::{HashMap};
use once_cell::sync::OnceCell;
use crate::events::Event;
use std::sync::Arc;

#[derive(Clone)]
pub struct SyncEvent {
    pub event: Arc<Event>,
    pub encoded: String,
}

impl SyncEvent {
    pub fn new(event: Event) -> SyncEvent {
        let encoded = serde_json::to_string(&event).unwrap();
        let event = Arc::new(event);
        SyncEvent { encoded, event }
    }
}

type BroadcastTable = RwLock<HashMap<Uuid, broadcast::Sender<SyncEvent>>>;

static BROADCAST_TABLE: OnceCell<BroadcastTable> = OnceCell::new();

pub fn get_broadcast_table() -> &'static BroadcastTable {
    BROADCAST_TABLE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn get_receiver(id: &Uuid) -> broadcast::Receiver<SyncEvent> {
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

pub type PreviewCache = Mutex<HashMap<Uuid, HashMap<Uuid, SyncEvent>>>;

static PREVIEW_CACHE: OnceCell<PreviewCache> = OnceCell::new();

pub fn get_preview_cache() -> &'static PreviewCache {
    PREVIEW_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

use crate::events::Event;
use once_cell::sync::OnceCell;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;

use crate::utils::timestamp;

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
        table.insert(*id, tx);
        rx
    }
}

pub struct ChannelCache {
    pub start_at: i64,
    pub events: VecDeque<Arc<SyncEvent>>,
    pub preview_map: HashMap<Uuid, Arc<SyncEvent>>,
    pub edition_map: HashMap<Uuid, Arc<SyncEvent>>,
}

pub struct Cache {
    pub channels: RwLock<HashMap<Uuid, Arc<Mutex<ChannelCache>>>>,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            channels: RwLock::new(HashMap::new()),
        }
    }

    pub async fn try_channel(&self, channel_id: &Uuid) -> Option<Arc<Mutex<ChannelCache>>> {
        let map = self.channels.read().await;
        map.get(&channel_id).cloned()
    }

    pub async fn channel(&self, channel_id: &Uuid) -> Arc<Mutex<ChannelCache>> {
        let map = self.channels.read().await;
        if let Some(cache) = map.get(&channel_id) {
            cache.clone()
        } else {
            drop(map);
            let cache = ChannelCache {
                start_at: timestamp(),
                events: VecDeque::new(),
                preview_map: HashMap::new(),
                edition_map: HashMap::new(),
            };
            let cache = Arc::new(Mutex::new(cache));
            let mut map = self.channels.write().await;
            map.insert(*channel_id, cache.clone());
            cache
        }
    }
}

static CACHE: OnceCell<Cache> = OnceCell::new();

pub fn get_cache() -> &'static Cache {
    CACHE.get_or_init(Cache::new)
}

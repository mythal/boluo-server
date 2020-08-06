use crate::cache;
use crate::events::context::{get_broadcast_table, get_heartbeat_map};
use crate::events::Event;
use crate::utils::timestamp;
use futures::StreamExt;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

pub fn start() {
    tokio::spawn(redis_clean());
    tokio::spawn(push_heartbeat());
    tokio::spawn(heartbeat_clean());
    tokio::spawn(broadcast_clean());
}

async fn redis_clean() {
    interval(Duration::from_secs(60 * 60 * 2))
        .for_each(|_| async {
            use redis::AsyncCommands;

            let mut cache = cache::conn().await;
            let keys: Vec<Vec<u8>> = if let Ok(keys) = cache.inner.keys(b"mailbox:*").await {
                keys
            } else {
                log::warn!("Failed to get redis keys.");
                return;
            };
            let before = timestamp() - 24 * 60 * 60 * 1000;
            for key in keys.into_iter() {
                if let Err(e) = cache.clear_before(&*key, before).await {
                    log::warn!("Failed to clear old events: {}", e);
                }
            }
            log::info!("Redis clean finished");
        })
        .await;
}

async fn push_heartbeat() {
    interval(Duration::from_secs(6))
        .for_each(|_| async {
            let map = get_heartbeat_map().lock().await;
            for (channel_id, heartbeat_map) in map.iter() {
                tokio::spawn(Event::push_heartbeat_map(*channel_id, heartbeat_map.clone()));
            }
        })
        .await;
}

async fn heartbeat_clean() {
    interval(Duration::from_secs(60 * 30))
        .for_each(|_| async {
            let now = timestamp();
            let mut map_ref = get_heartbeat_map().lock().await;
            let mut map = HashMap::new();
            let hour = 1000 * 60 * 60;
            std::mem::swap(&mut map, &mut *map_ref);
            for (channel_id, heartbeat_map) in map.into_iter() {
                let heartbeat_map: HashMap<Uuid, i64> = heartbeat_map
                    .into_iter()
                    .filter(|(_, time)| now - *time < hour)
                    .collect();
                if heartbeat_map.len() > 0 {
                    map_ref.insert(channel_id, heartbeat_map);
                }
            }
        })
        .await;
}

async fn broadcast_clean() {
    interval(Duration::from_secs(5 * 60))
        .for_each(|_| async {
            let mut broadcast_table = get_broadcast_table().write().await;
            broadcast_table.retain(|_, v| v.receiver_count() != 0);
            drop(broadcast_table);
            log::trace!("clean finished");
        })
        .await;
}

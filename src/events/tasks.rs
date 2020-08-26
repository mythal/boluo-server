use crate::context::debug;
use crate::events::context::{get_broadcast_table, get_heartbeat_map};
use crate::events::Event;
use crate::utils::timestamp;
use futures::StreamExt;
use std::collections::HashMap;
use std::mem::swap;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

pub fn start() {
    tokio::spawn(events_clean());
    tokio::spawn(push_heartbeat());
    tokio::spawn(heartbeat_clean());
    tokio::spawn(broadcast_clean());
}

async fn events_clean() {
    interval(Duration::from_secs(60 * 60 * 2))
        .for_each(|_| async {
            let mut next_map = HashMap::new();
            let before = timestamp() - 24 * 60 * 60 * 1000;
            let cache = super::context::get_cache().channels.read().await;
            for (id, channel) in cache.iter() {
                let mut empty = false;
                {
                    let mut channel = channel.lock().await;
                    while let Some(event) = channel.events.pop_front() {
                        if event.event.timestamp > before {
                            channel.events.push_front(event);
                            break;
                        }
                    }
                    let mut preview_map = HashMap::new();
                    let mut edition_map = HashMap::new();
                    swap(&mut preview_map, &mut channel.preview_map);
                    swap(&mut edition_map, &mut channel.edition_map);
                    channel.preview_map = preview_map
                        .into_iter()
                        .filter(|(_, preview)| preview.event.timestamp > before)
                        .collect();
                    channel.edition_map = edition_map
                        .into_iter()
                        .filter(|(_, edition)| edition.event.timestamp > before)
                        .collect();
                    channel.start_at = before;
                    if channel.events.is_empty() && channel.edition_map.is_empty() && channel.preview_map.is_empty() {
                        empty = true;
                    }
                }
                if !empty {
                    next_map.insert(*id, channel.clone());
                }
            }
            drop(cache);
            let mut cache = super::context::get_cache().channels.write().await;
            swap(&mut next_map, &mut *cache);
        })
        .await;
}

async fn push_heartbeat() {
    let duration = if debug() {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(6)
    };
    interval(duration)
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
                if !heartbeat_map.is_empty() {
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

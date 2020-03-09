use crate::utils::timestamp;
use futures::StreamExt;
use crate::events::context::get_broadcast_table;
use crate::cache;

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

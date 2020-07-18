use crate::error::CacheError;
use crate::utils::timestamp;
pub use redis::aio::ConnectionManager;
pub use redis::AsyncCommands;
use uuid::Uuid;

#[derive(Clone)]
pub struct Connection {
    pub inner: ConnectionManager,
}

impl Connection {
    fn new(inner: ConnectionManager) -> Connection {
        Connection { inner }
    }

    pub async fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, CacheError> {
        self.inner.get(key).await
    }

    pub async fn set(&mut self, key: &[u8], value: &[u8]) -> Result<(), CacheError> {
        self.inner.set(key, value).await
    }

    pub async fn remove(&mut self, key: &[u8]) -> Result<(), CacheError> {
        self.inner.del(key).await
    }

    pub async fn set_with_timestamp(&mut self, key: &[u8], value: &[u8]) -> Result<(), CacheError> {
        self.inner.zadd(key, value, timestamp()).await
    }

    pub async fn get_after(&mut self, key: &[u8], start: i64) -> Result<Vec<Vec<u8>>, CacheError> {
        self.inner.zrangebyscore(key, start, "+inf").await
    }

    pub async fn clear_before(&mut self, key: &[u8], end: i64) -> Result<(), CacheError> {
        self.inner.zrembyscore(key, "-inf", end).await
    }

    pub async fn get_min_time(&mut self, key: &[u8]) -> Result<i64, CacheError> {
        let (_, timestamp): (Vec<u8>, String) = self.inner.zrange_withscores(key, 0, 0).await?;
        Ok(timestamp.parse().expect("Unable to parse timestamp in the redis."))
    }

    pub async fn get_max_time(&mut self, key: &[u8]) -> Result<i64, CacheError> {
        let (_, timestamp): (Vec<u8>, String) = self.inner.zrevrange_withscores(key, 0, 0).await?;
        Ok(timestamp.parse().expect("Unable to parse timestamp in the redis."))
    }
}

#[derive(Clone)]
pub struct RedisFactory {
    client: redis::Client,
}

impl RedisFactory {
    pub fn new() -> RedisFactory {
        use std::env::var;
        let url = var("REDIS_URL").expect("Failed to load Redis URL");
        let client = redis::Client::open(&*url).unwrap();
        RedisFactory { client }
    }
}

/// Get cache database connection.
pub fn conn() -> Connection {
    use once_cell::sync::OnceCell;
    static FACTORY: OnceCell<Connection> = OnceCell::new();
    FACTORY
        .get_or_init(|| {
            use std::env::var;
            let url = var("REDIS_URL").expect("Failed to load Redis URL");
            let client = redis::Client::open(&*url).unwrap();
            let runtime = tokio::runtime::Handle::current();
            let connect_manager = runtime.block_on(client.get_tokio_connection_manager());
            Connection::new(connect_manager.expect("Unable to get tokio connection manager"))
        })
        .clone()
}

pub fn make_key(type_name: &[u8], id: &Uuid, field_name: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(type_name.len() + field_name.len() + 24);
    buffer.extend_from_slice(type_name);
    buffer.push(b':');
    buffer.extend_from_slice(&*id.as_bytes());
    buffer.push(b':');
    buffer.extend_from_slice(field_name);
    buffer
}

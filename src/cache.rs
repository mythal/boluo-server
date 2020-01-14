use crate::error::CacheError;
use crate::pool::{Connect, Factory, Pool};
use crate::utils::timestamp;
use async_trait::async_trait;
pub use redis::AsyncCommands;
use uuid::Uuid;

pub struct Connection {
    pub inner: redis::aio::Connection,
    broken: bool,
}

impl Connection {
    fn new(inner: redis::aio::Connection) -> Connection {
        let broken = false;
        Connection { inner, broken }
    }

    fn check<T>(&mut self, result: Result<T, CacheError>) -> Result<T, CacheError> {
        if let Err(ref e) = result {
            if e.is_connection_dropped() || e.is_connection_refusal() || e.is_timeout() || e.is_io_error() {
                self.broken = true;
            }
            log::error!("redis: {}", e);
        }
        result
    }

    pub async fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, CacheError> {
        let result = self.inner.get(key).await;
        self.check(result)
    }

    pub async fn set(&mut self, key: &[u8], value: &[u8]) -> Result<(), CacheError> {
        let result = self.inner.set(key, value).await;
        self.check(result)
    }

    pub async fn remove(&mut self, key: &[u8]) -> Result<(), CacheError> {
        let result = self.inner.del(key).await;
        self.check(result)
    }

    pub async fn set_with_time(&mut self, key: &[u8], value: &[u8]) -> Result<(), CacheError> {
        let result = self.inner.zadd(key, value, timestamp()).await;
        self.check(result)
    }

    pub async fn get_after(&mut self, key: &[u8], start: i64) -> Result<Vec<Vec<u8>>, CacheError> {
        let result: Result<Vec<Vec<u8>>, _> = self.inner.zrangebyscore(key, start, "+inf").await;
        self.check(result)
    }

    pub async fn clear_before(&mut self, key: &[u8], end: i64) -> Result<(), CacheError> {
        let result: Result<(), _> = self.inner.zrembyscore(key, "-inf", end).await;
        self.check(result)
    }

    pub async fn get_min_time(&mut self, key: &[u8]) -> Result<i64, CacheError> {
        let result: Result<(Vec<u8>, String), _> = self.inner.zrange_withscores(key, 0, 0).await;
        let (_, timestamp) = self.check(result)?;
        Ok(timestamp.parse().expect("Unexpected redis parse error."))
    }

    pub async fn get_max_time(&mut self, key: &[u8]) -> Result<i64, CacheError> {
        let result: Result<(Vec<u8>, String), _> = self.inner.zrevrange_withscores(key, 0, 0).await;
        let (_, timestamp) = self.check(result)?;
        Ok(timestamp.parse().expect("Unexpected redis parse error."))
    }
}

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

#[async_trait]
impl Factory for RedisFactory {
    type Output = Connection;

    fn is_broken(connection: &Connection) -> bool {
        connection.broken
    }

    async fn make(&self) -> Connection {
        let conn = self
            .client
            .get_async_connection()
            .await
            .expect("Unable to connect to the Redis server");
        Connection::new(conn)
    }
}

pub async fn get() -> Connect<RedisFactory> {
    use once_cell::sync::OnceCell;
    static POOL: OnceCell<Pool<RedisFactory>> = OnceCell::new();
    if let Some(pool) = POOL.get() {
        pool.get().await
    } else {
        let factory = RedisFactory::new();
        let pool = Pool::with_num(8, factory).await;
        POOL.get_or_init(move || pool).get().await
    }
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

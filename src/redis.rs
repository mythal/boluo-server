use crate::pool::{Connect, Factory, Pool};
use async_trait::async_trait;
pub use redis::AsyncCommands;
use thiserror::Error;
use uuid::Uuid;

pub struct Connection {
    inner: redis::aio::Connection,
    broken: bool,
}

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("redis error")]
    Redis(#[from] redis::RedisError),
}

impl Connection {
    fn new(inner: redis::aio::Connection) -> Connection {
        let broken = false;
        Connection { inner, broken }
    }

    fn handle<T>(&mut self, result: Result<T, redis::RedisError>) -> Result<T, QueryError> {
        if let Err(ref e) = result {
            if e.is_connection_dropped() || e.is_connection_refusal() || e.is_timeout() || e.is_io_error() {
                self.broken = true;
            }
            log::error!("redis: {}", e);
        }
        Ok(result?)
    }

    pub async fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, QueryError> {
        let result = self.inner.get(key).await;
        self.handle(result)
    }

    pub async fn set(&mut self, key: &[u8], value: &[u8]) -> Result<(), QueryError> {
        let result = self.inner.set(key, value).await;
        self.handle(result)
    }

    pub async fn remove(&mut self, key: &[u8]) -> Result<(), QueryError> {
        let result = self.inner.del(key).await;
        self.handle(result)
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

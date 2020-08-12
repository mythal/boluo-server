use crate::error::CacheError;
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

    pub async fn set_with_expiration(&mut self, key: &[u8], value: &[u8], seconds: usize) -> Result<(), CacheError> {
        self.inner.set_ex(key, value, seconds).await
    }

    pub async fn remove(&mut self, key: &[u8]) -> Result<(), CacheError> {
        self.inner.del(key).await
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
pub async fn conn() -> Connection {
    use once_cell::sync::OnceCell;
    static FACTORY: OnceCell<Connection> = OnceCell::new();
    if let Some(connecion) = FACTORY.get() {
        connecion.clone()
    } else {
        use std::env::var;
        let url = var("REDIS_URL").expect("Failed to load Redis URL");
        let connection_manager = redis::Client::open(&*url)
            .expect("Unable to open redis")
            .get_tokio_connection_manager()
            .await
            .expect("Unable to get tokio connection manager");
        let connection = Connection::new(connection_manager);
        if let Err(_) = FACTORY.set(connection.clone()) {
            panic!("Unable to set redis `FACTORY`.")
        }
        connection
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

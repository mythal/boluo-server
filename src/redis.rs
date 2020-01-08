use crate::pool::{Connect, Factory, Pool};
use async_trait::async_trait;
use redis::aio::Connection;
pub use redis::AsyncCommands;

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

    async fn make(&self) -> Connection {
        self.client
            .get_async_connection()
            .await
            .expect("unable connect to redis")
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

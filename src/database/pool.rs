use super::Client;
use crate::pool::{Connect, Factory, Pool};
use async_trait::async_trait;
use once_cell::sync::OnceCell;

pub struct PostgresFactory {
    config: tokio_postgres::Config,
}

impl PostgresFactory {
    pub fn new() -> PostgresFactory {
        use std::env::var;
        let config = var("DATABASE_URL")
            .expect("Failed to load Postgres URL")
            .parse()
            .unwrap();
        PostgresFactory { config }
    }
}

#[async_trait]
impl Factory for PostgresFactory {
    type Output = Client;

    fn is_broken(client: &Client) -> bool {
        client.is_broken()
    }

    async fn make(&self) -> Client {
        Client::with_config(&self.config).await
    }
}

pub async fn get() -> Connect<PostgresFactory> {
    static POOL: OnceCell<Pool<PostgresFactory>> = OnceCell::new();
    if let Some(pool) = POOL.get() {
        pool.get().await
    } else {
        let factory = PostgresFactory::new();
        let pool = Pool::with_num(10, factory).await;
        POOL.get_or_init(move || pool).get().await
    }
}

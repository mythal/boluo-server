use super::Client;
use crate::error::DbError;
use crate::pool::{Connect, Factory, Pool};
use async_trait::async_trait;
use once_cell::sync::OnceCell;

#[derive(Clone)]
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
    type Error = DbError;

    fn is_broken(client: &Client) -> bool {
        client.is_broken()
    }

    async fn make(&self) -> Result<Client, DbError> {
        Client::with_config(&self.config).await
    }
}

pub async fn get() -> Result<Connect<PostgresFactory>, DbError> {
    static FACTORY: OnceCell<PostgresFactory> = OnceCell::new();
    let factory = FACTORY.get_or_init(PostgresFactory::new);
    static POOL: OnceCell<Pool<PostgresFactory>> = OnceCell::new();
    if let Some(pool) = POOL.get() {
        pool.get().await
    } else {
        let pool = Pool::with_num(10, factory.clone()).await;
        POOL.get_or_init(move || pool).get().await
    }
}

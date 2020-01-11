use std::collections::HashMap;
use std::env;
use std::hash::BuildHasher;

use thiserror::Error;
pub use tokio_postgres::types::{ToSql, Type as SqlType};

use async_trait::async_trait;

pub mod pool;
pub mod query;

pub use pool::get;

pub type DbError = tokio_postgres::Error;

#[async_trait]
pub trait Querist: Send {
    async fn query(
        &mut self,
        key: query::Key,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>, DbError>;

    async fn execute(&mut self, key: query::Key, params: &[&(dyn ToSql + Sync)]) -> Result<u64, DbError>;

    async fn fetch(
        &mut self,
        key: query::Key,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<tokio_postgres::Row, FetchError> {
        self.query(key, params)
            .await?
            .into_iter()
            .next()
            .ok_or(FetchError::NoSuchRecord)
    }

    async fn create(
        &mut self,
        key: query::Key,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<tokio_postgres::Row, CreationError> {
        self.query(key, params)
            .await?
            .into_iter()
            .next()
            .ok_or(CreationError::EmptyResult)
    }
}

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("unknown query error")]
    QueryFail(#[from] DbError),
    #[error("no such record")]
    NoSuchRecord,
    #[error("no permission to access record")]
    NoPermission,
}

#[derive(Error, Debug)]
pub enum CreationError {
    #[error("unknown query error")]
    QueryFail(#[from] DbError),
    #[error("record already exists")]
    EmptyResult,
    #[error("validation failed: {0}")]
    ValidationFail(String),
}

pub fn get_postgres_url() -> String {
    let key = "DATABASE_URL";
    env::var(key).expect("Failed to load Postgres connect URL")
}

pub struct CrcBuilder;

impl BuildHasher for CrcBuilder {
    type Hasher = crc32fast::Hasher;

    fn build_hasher(&self) -> crc32fast::Hasher {
        crc32fast::Hasher::new()
    }
}

pub type PrepareMap = HashMap<query::Key, tokio_postgres::Statement, CrcBuilder>;

pub struct Client {
    pub client: tokio_postgres::Client,
    broken: bool,
    prepared: PrepareMap,
}

impl Client {
    pub async fn new() -> Client {
        Client::with_config(&get_postgres_url().parse().unwrap()).await
    }

    pub fn is_broken(&self) -> bool {
        self.broken
    }

    fn mark_broken(&mut self) {
        self.broken = true;
        log::warn!("A postgres connection was broken.");
    }

    async fn prepare(client: &mut tokio_postgres::Client) -> PrepareMap {
        let mut map = HashMap::with_capacity_and_hasher(20, CrcBuilder);
        for query in query::ALL_QUERY.iter() {
            let statement = client.prepare_typed(query.source, query.types).await.unwrap();
            map.insert(query.key, statement);
        }
        map
    }

    pub async fn with_config(config: &tokio_postgres::Config) -> Client {
        let (mut client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(connection);
        let prepared = Client::prepare(&mut client).await;
        let broken = false;
        Client {
            client,
            prepared,
            broken,
        }
    }

    pub async fn transaction(&'_ mut self) -> Result<Transaction<'_>, DbError> {
        if self.client.is_closed() {
            self.mark_broken()
        }
        let transaction = self.client.transaction().await?;
        let prepared = &self.prepared;
        Ok(Transaction { transaction, prepared })
    }
}

#[async_trait]
impl Querist for Client {
    async fn query(
        &mut self,
        key: query::Key,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>, DbError> {
        let statement = self.prepared.get(&key).expect("Query not found");
        let result = self.client.query(statement, params).await;
        if result.is_err() && self.client.is_closed() {
            self.mark_broken();
        }
        result
    }

    async fn execute(&mut self, key: query::Key, params: &[&(dyn ToSql + Sync)]) -> Result<u64, DbError> {
        let statement = self.prepared.get(&key).expect("Query not found");
        let result = self.client.execute(statement, params).await;
        if result.is_err() && self.client.is_closed() {
            self.mark_broken();
        }
        result
    }
}

pub struct Transaction<'a> {
    pub transaction: tokio_postgres::Transaction<'a>,
    prepared: &'a PrepareMap,
}

impl<'a> Transaction<'a> {
    pub async fn commit(self) -> Result<(), DbError> {
        self.transaction.commit().await
    }

    pub async fn rollback(self) -> Result<(), DbError> {
        self.transaction.rollback().await
    }
}

#[async_trait]
impl Querist for Transaction<'_> {
    async fn query(
        &mut self,
        key: query::Key,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>, DbError> {
        let statement = self.prepared.get(&key).expect("Query not found");
        self.transaction.query(statement, params).await
    }

    async fn execute(&mut self, key: query::Key, params: &[&(dyn ToSql + Sync)]) -> Result<u64, DbError> {
        let statement = self.prepared.get(&key).expect("Query not found");
        self.transaction.execute(statement, params).await
    }
}

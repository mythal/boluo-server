use std::collections::{HashMap, VecDeque};
use std::env;
use std::hash::BuildHasher;
use std::sync::{Arc, Mutex};

use postgres::types::ToSql;
pub use postgres::types::Type as SqlType;
use thiserror::Error;

pub trait Querist {
    fn query(&mut self, key: query::Key, params: &[&(dyn ToSql + Sync)])
             -> Result<Vec<postgres::Row>, postgres::Error>;

    fn fetch(&mut self, key: query::Key, params: &[&(dyn ToSql + Sync)]) -> Result<postgres::Row, FetchError> {
        self.query(key, params)?
            .into_iter()
            .next()
            .ok_or(FetchError::NoSuchRecord)
    }

    fn create(&mut self, key: query::Key, params: &[&(dyn ToSql + Sync)]) -> Result<postgres::Row, CreationError> {
        self.query(key, params)?
            .into_iter()
            .next()
            .ok_or(CreationError::AlreadyExists)
    }
}

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("unknown query error")]
    QueryFail(#[from] postgres::Error),
    #[error("no such record")]
    NoSuchRecord,
}

#[derive(Error, Debug)]
pub enum CreationError {
    #[error("unknown query error")]
    QueryFail(#[from] postgres::Error),
    #[error("record already exists")]
    AlreadyExists,
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

pub type PrepareMap = HashMap<query::Key, postgres::Statement, CrcBuilder>;

pub struct Client {
    pub client: postgres::Client,
    prepared: PrepareMap,
}

pub mod query;

impl Client {
    pub fn new() -> Client {
        Client::with_config(get_postgres_url().parse().unwrap())
    }

    fn prepare(client: &mut postgres::Client) -> PrepareMap {
        let mut map = HashMap::with_capacity_and_hasher(20, CrcBuilder);
        for query in query::ALL_QUERY.iter() {
            let statement = client.prepare_typed(query.source, query.types).unwrap();
            map.insert(query.key, statement);
        }
        map
    }

    pub fn with_config(config: postgres::Config) -> Client {
        let mut client = config.connect(postgres::NoTls).unwrap();
        let prepared = Client::prepare(&mut client);
        Client { client, prepared }
    }

    pub fn transaction(&mut self) -> Result<Transaction, postgres::Error> {
        let transaction = self.client.transaction()?;
        let prepared = &self.prepared;
        Ok(Transaction { transaction, prepared })
    }
}

impl Querist for Client {
    fn query(
        &mut self,
        key: query::Key,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<postgres::Row>, postgres::Error> {
        let statement = self.prepared.get(&key).expect("Query not found");
        self.client.query(statement, params)
    }
}

pub struct Transaction<'a> {
    pub transaction: postgres::Transaction<'a>,
    prepared: &'a PrepareMap,
}

impl<'a> Transaction<'a> {
    fn commit(self) -> Result<(), postgres::Error> {
        self.transaction.commit()
    }

    fn rollback(self) -> Result<(), postgres::Error> {
        self.transaction.rollback()
    }
}

impl<'a> Querist for Transaction<'a> {
    fn query(
        &mut self,
        key: query::Key,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<postgres::Row>, postgres::Error> {
        let statement = self.prepared.get(&key).expect("Query not found");
        self.transaction.query(statement, params)
    }
}

struct InternalPool {
    conns: VecDeque<Client>,
    num: u32,
}

struct SharedPool {
    config: postgres::Config,
    inner: Mutex<InternalPool>,
}

pub struct Pool {
    inner: Arc<SharedPool>,
}

impl Pool {
    pub fn new() -> Pool {
        let config: postgres::Config = env::var("POSTGRES_URL")
            .expect("Failed to load Postgres connect URL.")
            .parse()
            .unwrap();
        let internal_pool = InternalPool {
            conns: VecDeque::new(),
            num: 0,
        };
        let shared_pool = SharedPool {
            inner: Mutex::new(internal_pool),
            config,
        };
        Pool {
            inner: Arc::new(shared_pool),
        }
    }
}

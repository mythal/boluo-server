use std::collections::{HashMap, VecDeque};
use std::env;
use std::hash::BuildHasher;
use std::sync::{Arc, Mutex};

use postgres::types::Type;
pub use postgres::types::Type as SqlType;
use thiserror::Error;

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


pub struct Client {
    pub client: postgres::Client,
    prepared: HashMap<&'static str, postgres::Statement, CrcBuilder>,
}

impl Client {
    pub fn new() -> Client {
        Client::with_config(get_postgres_url().parse().unwrap())
    }

    pub fn with_config(config: postgres::Config) -> Client {
        let client = config.connect(postgres::NoTls).unwrap();
        let prepared = HashMap::with_capacity_and_hasher(8, CrcBuilder);
        Client { client, prepared }
    }

    pub fn prepare(&mut self, query: &'static str, types: &[Type]) -> postgres::Statement {
        if let Some(statement) = self.prepared.get(query) {
            statement.clone()
        } else {
            let statement = self.client.prepare_typed(query, types).unwrap();
            self.prepared.insert(query, statement.clone());
            statement
        }
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
        use std::str::FromStr;

        let config: postgres::Config = env::var("POSTGRES_URL")
            .expect("Failed to load Postgres connect URL.")
            .parse()
            .unwrap();
        let internal_pool = InternalPool { conns: VecDeque::new(), num: 0 };
        let shared_pool = SharedPool { inner: Mutex::new(internal_pool), config };
        Pool { inner: Arc::new(shared_pool) }
    }
}

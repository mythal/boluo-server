use std::collections::VecDeque;
use std::env;
use std::mem::drop;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Weak};

use futures::channel::oneshot;
use tokio::sync::{Mutex, MutexGuard};

use crate::database::Client;

static UNRELEASED: AtomicIsize = AtomicIsize::new(0);

pub struct Connect {
    connect: Option<Client>,
    pool: Weak<SharedPool>,
}

impl Connect {
    pub async fn release(mut self) {
        let pool = self.pool.upgrade();
        if let Some(pool) = pool {
            let mut pool = pool.inner.lock().await;
            pool.put_back(self.connect.take().unwrap());
        }
    }
}

impl Deref for Connect {
    type Target = Client;

    fn deref(&self) -> &Client {
        self.connect.as_ref().unwrap()
    }
}

impl DerefMut for Connect {
    fn deref_mut(&mut self) -> &mut Client {
        self.connect.as_mut().unwrap()
    }
}

impl Drop for Connect {
    fn drop(&mut self) {
        UNRELEASED.fetch_add(1, Ordering::Relaxed);
    }
}

struct InternalPool {
    waiters: VecDeque<oneshot::Sender<Client>>,
    conns: VecDeque<Client>,
    num: usize,
}

impl InternalPool {
    fn put_back(&mut self, mut connect: Client) {
        while let Some(waiter) = self.waiters.pop_front() {
            if let Err(returned) = waiter.send(connect) {
                connect = returned;
            } else {
                return;
            }
        }
        self.conns.push_back(connect);
    }
}

struct SharedPool {
    config: tokio_postgres::Config,
    inner: Mutex<InternalPool>,
}

#[derive(Clone)]
pub struct Pool {
    inner: Arc<SharedPool>,
}

impl Pool {
    pub async fn with_num(num: usize) -> Pool {
        let config: tokio_postgres::Config = env::var("DATABASE_URL")
            .expect("Failed to load Postgres connect URL")
            .parse()
            .unwrap();

        let mut conns: VecDeque<Client> = VecDeque::with_capacity(num);
        for _ in 0..num {
            conns.push_back(Client::with_config(&config).await);
        }
        let waiters = VecDeque::new();
        let internal_pool = InternalPool { waiters, conns, num };
        let inner = Mutex::new(internal_pool);
        let shared_pool = SharedPool { inner, config };
        Pool {
            inner: Arc::new(shared_pool),
        }
    }

    pub async fn get(&self) -> Connect {
        let mut internal: MutexGuard<InternalPool> = self.inner.inner.lock().await;
        let pool = Arc::downgrade(&self.inner);
        if let Some(client) = internal.conns.pop_front() {
            Connect {
                connect: Some(client),
                pool,
            }
        } else if UNRELEASED.fetch_sub(1, Ordering::Relaxed) <= 0 {
            UNRELEASED.fetch_add(1, Ordering::Relaxed);
            let (tx, rx) = oneshot::channel::<Client>();
            internal.waiters.push_back(tx);
            drop(internal);
            Connect {
                connect: Some(rx.await.unwrap()),
                pool,
            }
        } else {
            let new = Client::with_config(&self.inner.config).await;
            Connect {
                connect: Some(new),
                pool,
            }
        }
    }
}

#[tokio::test]
async fn pool_test() {
    let pool = Pool::with_num(10).await;
    let db = pool.get().await;
    db.release().await;
}

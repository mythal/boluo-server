use std::collections::VecDeque;
use std::env;
use std::future::Future;
use std::mem::drop;
use std::sync::Arc;

use futures::channel::oneshot;
use tokio::sync::{Mutex, MutexGuard};

use crate::database::Client;

struct InternalPool {
    waiters: VecDeque<oneshot::Sender<Client>>,
    conns: VecDeque<Client>,
    num: usize,
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

    async fn get_client(&self) -> Client {
        let mut internal: MutexGuard<InternalPool> = self.inner.inner.lock().await;
        if let Some(client) = internal.conns.pop_front() {
            client
        } else {
            let (tx, rx) = oneshot::channel::<Client>();
            internal.waiters.push_back(tx);
            drop(internal);
            rx.await.unwrap()
        }
    }

    async fn put_client(&self, mut client: Client) {
        if client.client.is_closed() {
            client = Client::with_config(&self.inner.config).await;
        }
        let mut internal: MutexGuard<InternalPool> = self.inner.inner.lock().await;
        while let Some(waiter) = internal.waiters.pop_front() {
            if let Err(returned) = waiter.send(client) {
                client = returned;
            } else {
                return;
            }
        }
        internal.conns.push_back(client);
    }

    pub async fn run<F, R>(&self, f: F)
        where
            F: FnOnce(Client) -> R,
            R: Future<Output=Client>,
    {
        let client = f(self.get_client().await).await;
        self.put_client(client).await;
    }
}

#[tokio::test]
async fn pool_test() {
    let pool = Pool::with_num(10).await;
    pool.run(|conn| async { conn }).await;
}

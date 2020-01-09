use std::collections::VecDeque;
use std::mem::drop;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use futures::channel::{mpsc, oneshot};
use futures::lock::Mutex;

pub struct Connect<F: Factory> {
    connect: Option<F::Output>,
    pool: Weak<SharedPool<F>>,
}

impl<F: Factory> Connect<F> {}

impl<F: Factory> Deref for Connect<F> {
    type Target = F::Output;

    fn deref(&self) -> &F::Output {
        self.connect.as_ref().unwrap()
    }
}

impl<F: Factory> DerefMut for Connect<F> {
    fn deref_mut(&mut self) -> &mut F::Output {
        self.connect.as_mut().unwrap()
    }
}

impl<F: Factory> Drop for Connect<F> {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            let mut tx = pool.connect_sender.clone();
            tx.try_send(self.connect.take().unwrap()).ok();
        }
    }
}

struct InternalPool<C> {
    waiters: VecDeque<oneshot::Sender<C>>,
    conns: VecDeque<C>,
    num: usize,
}

impl<C> InternalPool<C> {
    fn put_back(&mut self, mut connect: C) {
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

struct SharedPool<F: Factory> {
    factory: F,
    inner: Mutex<InternalPool<F::Output>>,
    connect_sender: mpsc::Sender<F::Output>,
}

#[derive(Clone)]
pub struct Pool<F: Factory> {
    inner: Arc<SharedPool<F>>,
}

impl<F: Factory> Pool<F> {
    async fn recycle(shared: Weak<SharedPool<F>>, mut rx: mpsc::Receiver<F::Output>) {
        use futures::stream::StreamExt;
        while let Some(mut conn) = StreamExt::next(&mut rx).await {
            if let Some(shared) = shared.upgrade() {
                if !F::check(&conn) {
                    conn = shared.factory.make().await;
                }
                let mut pool = shared.inner.lock().await;
                pool.put_back(conn);
            } else {
                break;
            }
        }
    }

    pub async fn with_num(num: usize, factory: F) -> Pool<F> {
        use tokio::task::spawn;
        let mut conns: VecDeque<F::Output> = VecDeque::with_capacity(num);
        for _ in 0..num {
            conns.push_back(factory.make().await);
        }
        let waiters = VecDeque::new();
        let internal_pool = InternalPool { waiters, conns, num };

        let (tx, rx) = mpsc::channel::<F::Output>(num);

        let shared_pool = Arc::new(SharedPool {
            inner: Mutex::new(internal_pool),
            factory,
            connect_sender: tx,
        });

        spawn(Pool::recycle(Arc::downgrade(&shared_pool), rx));
        Pool { inner: shared_pool }
    }

    pub async fn get(&self) -> Connect<F> {
        let mut internal = self.inner.inner.lock().await;
        let pool = Arc::downgrade(&self.inner);
        if let Some(conn) = internal.conns.pop_front() {
            Connect {
                connect: Some(conn),
                pool,
            }
        } else {
            let (tx, rx) = oneshot::channel::<F::Output>();
            internal.waiters.push_back(tx);
            drop(internal);
            Connect {
                connect: Some(rx.await.unwrap()),
                pool,
            }
        }
    }
}

#[tokio::test]
async fn pool_test() {
    use crate::database::pool::PostgresFactory;
    let config = PostgresFactory::new();
    let pool = Pool::with_num(10, config).await;
    let _db = pool.get().await;
}

#[async_trait]
pub trait Factory: Send + Sync + 'static {
    type Output: Send;

    fn check(connection: &Self::Output) -> bool;
    async fn make(&self) -> Self::Output;
}

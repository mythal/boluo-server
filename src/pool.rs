use std::collections::VecDeque;
use std::mem::drop;
use std::ops::{Deref, DerefMut, Drop};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use futures::channel::oneshot;
use tokio::sync::{Mutex, MutexGuard};

pub struct Connect<C, F>
where
    F: Factory<Output = C>,
{
    connect: Option<C>,
    pool: Weak<SharedPool<C, F>>,
}

impl<C, F> Connect<C, F>
where
    F: Factory<Output = C>,
{
    pub async fn release(mut self) {
        let pool = self.pool.upgrade();
        if let Some(pool) = pool {
            let mut pool = pool.inner.lock().await;
            pool.put_back(self.connect.take().unwrap());
        }
    }
}

impl<C, F> Deref for Connect<C, F>
where
    F: Factory<Output = C>,
{
    type Target = C;

    fn deref(&self) -> &C {
        self.connect.as_ref().unwrap()
    }
}

impl<C, F> DerefMut for Connect<C, F>
where
    F: Factory<Output = C>,
{
    fn deref_mut(&mut self) -> &mut C {
        self.connect.as_mut().unwrap()
    }
}

impl<C, F> Drop for Connect<C, F>
where
    F: Factory<Output = C>,
{
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            pool.unreleased.fetch_add(1, Ordering::Relaxed);
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

struct SharedPool<C, F: Factory<Output = C>> {
    factory: F,
    inner: Mutex<InternalPool<C>>,
    unreleased: AtomicIsize,
}

#[derive(Clone)]
pub struct Pool<C, F: Factory<Output = C>> {
    inner: Arc<SharedPool<C, F>>,
}

impl<C, F> Pool<C, F>
where
    F: Factory<Output = C>,
{
    pub async fn with_num(num: usize, factory: F) -> Pool<C, F> {
        let mut conns: VecDeque<C> = VecDeque::with_capacity(num);
        for _ in 0..num {
            conns.push_back(factory.make().await);
        }
        let waiters = VecDeque::new();
        let internal_pool = InternalPool { waiters, conns, num };
        let shared_pool = SharedPool {
            inner: Mutex::new(internal_pool),
            factory,
            unreleased: AtomicIsize::new(0),
        };
        Pool {
            inner: Arc::new(shared_pool),
        }
    }

    pub async fn get(&self) -> Connect<C, F> {
        let mut internal: MutexGuard<InternalPool<C>> = self.inner.inner.lock().await;
        let pool = Arc::downgrade(&self.inner);
        if let Some(conn) = internal.conns.pop_front() {
            Connect {
                connect: Some(conn),
                pool,
            }
        } else if self.inner.unreleased.fetch_sub(1, Ordering::Relaxed) <= 0 {
            self.inner.unreleased.fetch_add(1, Ordering::Relaxed);
            let (tx, rx) = oneshot::channel::<C>();
            internal.waiters.push_back(tx);
            drop(internal);
            Connect {
                connect: Some(rx.await.unwrap()),
                pool,
            }
        } else {
            let new: C = self.inner.factory.make().await;
            Connect {
                connect: Some(new),
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
    let db = pool.get().await;
    db.release().await;
}

#[async_trait]
pub trait Factory {
    type Output;

    async fn make(&self) -> Self::Output;
}

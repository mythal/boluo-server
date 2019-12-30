use std::env;

use once_cell::sync::OnceCell;

use crate::database::pool::Pool;

static POOL: OnceCell<Pool> = OnceCell::new();
static DEBUG: OnceCell<bool> = OnceCell::new();
static NOT_INIT: &str = "not initialized";

pub fn pool() -> &'static Pool {
    POOL.get().expect(NOT_INIT)
}

fn env_bool<T: AsRef<str>>(s: T) -> bool {
    let s = s.as_ref().trim();
    !(s.is_empty() || s == "0" || s.to_ascii_lowercase() == "false")
}

pub fn debug() -> bool {
    *DEBUG.get_or_init(|| env::var("DEBUG").map(env_bool).unwrap_or(false))
}

pub async fn init() {
    POOL.set(Pool::with_num(10).await).ok().unwrap();
}

use std::{env, path::PathBuf, path::Path};

use once_cell::sync::OnceCell;

static DEBUG: OnceCell<bool> = OnceCell::new();
static SYSTEMD: OnceCell<bool> = OnceCell::new();
static SECRET: OnceCell<String> = OnceCell::new();

fn env_bool<T: AsRef<str>>(s: T) -> bool {
    let s = s.as_ref().trim();
    !(s.is_empty() || s == "0" || s.to_ascii_lowercase() == "false")
}

pub fn debug() -> bool {
    *DEBUG.get_or_init(|| env::var("DEBUG").map(env_bool).unwrap_or(false))
}

pub fn secret() -> &'static str {
    &*SECRET.get_or_init(|| env::var("SECRET").unwrap())
}

pub fn is_systemd() -> bool {
    *SYSTEMD.get_or_init(|| env::var("SYSTEMD").map(env_bool).unwrap_or(false))
}

static MEDIA_PATH: OnceCell<PathBuf> = OnceCell::new();

pub fn media_path() -> &'static Path {
    MEDIA_PATH.get_or_init(|| PathBuf::from(env::var("MEDIA_PATH").unwrap_or("media".to_string()))) 
}

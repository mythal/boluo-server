use once_cell::sync::OnceCell;
use ring::hmac;
use ring::rand::SecureRandom;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

macro_rules! regex {
    ($pattern: expr) => {{
        use once_cell::sync::OnceCell;
        use regex::Regex;
        static CELL: OnceCell<Regex> = OnceCell::new();
        CELL.get_or_init(|| Regex::new($pattern).unwrap())
    }};
}

pub fn now_unix_duration() -> Duration {
    use std::time::UNIX_EPOCH;

    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).expect("SystemTime before UNIX EPOCH!")
}

pub fn id() -> Uuid {
    use uuid::v1::Context as UuidContext;
    use uuid::v1::Timestamp;

    static NODE_ID: OnceCell<[u8; 6]> = OnceCell::new();
    let node_id = NODE_ID.get_or_init(|| {
        let rng = ring::rand::SystemRandom::new();
        let mut id = [0u8; 6];
        rng.fill(&mut id).unwrap();
        id
    });
    let now = now_unix_duration();
    static CONTEXT: UuidContext = UuidContext::new(0);
    let timestamp = Timestamp::from_unix(&CONTEXT, now.as_secs(), now.subsec_nanos());
    Uuid::new_v1(timestamp, node_id).expect("failed to generate UUID")
}

fn key() -> &'static hmac::Key {
    use crate::context::secret;
    use ring::digest;
    static KEY: OnceCell<hmac::Key> = OnceCell::new();
    KEY.get_or_init(|| {
        let digest = digest::digest(&digest::SHA256, secret().as_bytes());
        hmac::Key::new(hmac::HMAC_SHA256, digest.as_ref())
    })
}

pub fn sign(message: &str) -> hmac::Tag {
    hmac::sign(key(), message.as_bytes())
}

pub fn verify(message: &str, signature: &str) -> Option<()> {
    let signature = base64::decode(signature).ok()?;
    hmac::verify(key(), message.as_bytes(), &*signature).ok()
}

pub fn timestamp() -> i64 {
    use chrono::Local;
    Local::now().timestamp_millis()
}

pub fn inner_map<T, E, U, F: Fn(T) -> U>(x: Result<Option<T>, E>, mapper: F) -> Result<Option<U>, E> {
    x.map(|y| y.map(mapper))
}

#[test]
fn test_sign() {
    let message = "hello, world";
    let signature = sign(message);
    let signature = base64::encode(&signature);
    verify(message, &*signature).unwrap();
}

use once_cell::sync::OnceCell;
use ring::hmac;
use ring::rand::SecureRandom;
use uuid::Uuid;

macro_rules! regex {
    ($pattern: expr) => {{
        use once_cell::sync::OnceCell;
        use regex::Regex;
        static CELL: OnceCell<Regex> = OnceCell::new();
        CELL.get_or_init(|| Regex::new($pattern).unwrap())
    }};
}

pub fn id() -> Uuid {
    use std::time::SystemTime;
    use uuid::v1::Context as UuidContext;
    use uuid::v1::Timestamp;

    static NODE_ID: OnceCell<[u8; 6]> = OnceCell::new();
    let node_id = NODE_ID.get_or_init(|| {
        let rng = ring::rand::SystemRandom::new();
        let mut id = [0u8; 6];
        rng.fill(&mut id).unwrap();
        id
    });
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!");
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

pub fn sign(message: &str) -> String {
    let signed = hmac::sign(key(), message.as_bytes());
    base64::encode(&signed)
}

pub fn verify(message: &str, sign: &str) -> Option<()> {
    let sign = base64::decode(sign).ok()?;
    hmac::verify(key(), message.as_bytes(), &*sign).ok()
}

#[test]
fn test_sign() {
    let message = "hello, world";
    let signed = sign(message);
    verify(message, &*signed).unwrap();
}

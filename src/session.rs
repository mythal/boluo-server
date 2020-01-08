use crate::utils::{id, sign, verify};
use futures::lock::Mutex;
use once_cell::sync::OnceCell;
use regex::Regex;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct Session {
    pub key: Uuid,
    pub user_id: Uuid,
}

impl Session {
    pub fn new(user_id: &Uuid) -> Session {
        Session {
            key: id(),
            user_id: *user_id,
        }
    }

    pub fn token(&self) -> String {
        // [body (base64)].[sign]
        let mut buffer = String::with_capacity(64);
        base64::encode_config_buf(self.key.as_bytes(), base64::STANDARD, &mut buffer);
        let signature = sign(&*buffer);
        buffer.push('.');
        base64::encode_config_buf(&signature, base64::STANDARD, &mut buffer);
        buffer
    }

    pub fn verify(token: &str) -> Result<Uuid, Unauthenticated> {
        use Unauthenticated::{AuthFailed, Unexpected};
        let mut iter = token.split('.');
        let token = iter.next().ok_or(Unexpected)?;
        let signature = iter.next().ok_or(Unexpected)?;
        verify(token, signature).ok_or(AuthFailed("Mismatched data and signature."))?;
        let token = base64::decode(token).map_err(|_| Unexpected)?;
        Uuid::from_slice(token.as_slice()).map_err(|_| Unexpected)
    }
}

#[test]
fn test_session_sign() {
    let user_id = Uuid::new_v4();
    let session = Session::new(&user_id);
    let token = session.token();
    assert!(Session::verify("").is_err());
    let key = Session::verify(&*token).unwrap();
    assert_eq!(key, session.key);
}

pub struct SessionMap {
    inner: Mutex<HashMap<Uuid, Session>>,
}

static SESSION_MAP: OnceCell<SessionMap> = OnceCell::new();

impl SessionMap {
    pub fn new() -> SessionMap {
        SessionMap {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub async fn start(&self, user_id: &Uuid) -> Session {
        let mut inner = self.inner.lock().await;
        let session = Session::new(user_id);
        let key = session.key;
        inner.insert(key, session.clone());
        session
    }

    pub async fn get_session(&self, key: &Uuid) -> Option<Session> {
        let inner = self.inner.lock().await;
        inner.get(key).map(Clone::clone)
    }

    pub fn get() -> &'static SessionMap {
        SESSION_MAP.get_or_init(SessionMap::new)
    }
}

fn get_cookie(value: &hyper::header::HeaderValue) -> Option<&str> {
    static COOKIE_PATTERN: OnceCell<Regex> = OnceCell::new();
    let cookie_pattern = COOKIE_PATTERN.get_or_init(|| Regex::new(r#"\bsession=([^;]+)"#).unwrap());
    let value = value.to_str().ok()?;
    cookie_pattern.captures(value)?.get(1).map(|m| m.as_str())
}

async fn get_session(token: &str) -> Result<Session, Unauthenticated> {
    use Unauthenticated::AuthFailed;
    let session_key = Session::verify(token)?;
    SessionMap::get()
        .get_session(&session_key)
        .await
        .ok_or_else(|| AuthFailed("Session key not found."))
}

#[derive(Debug)]
pub enum Unauthenticated {
    ParseFailed(&'static str),
    AuthFailed(&'static str),
    Unexpected,
}

pub async fn authenticate(req: &hyper::Request<hyper::Body>) -> Result<Session, Unauthenticated> {
    use hyper::header::{HeaderValue, AUTHORIZATION, COOKIE};

    let headers = req.headers();
    let authorization = headers.get(AUTHORIZATION).map(HeaderValue::to_str);

    if let Some(Ok(token)) = authorization {
        return get_session(token).await;
    }
    let cookie_value = headers
        .get(COOKIE)
        .and_then(get_cookie)
        .ok_or_else(|| Unauthenticated::ParseFailed("Can't retrieve session cookie."))?;
    get_session(cookie_value).await
}

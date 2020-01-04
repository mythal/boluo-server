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
        // [sign].[body (base64)]
        let body = base64::encode(self.key.as_bytes());
        let mut token = sign(&*body);
        token.push('.');
        token.push_str(&*body);
        token
    }

    pub fn verify(token: &str) -> Option<Uuid> {
        let mut iter = token.split('.');
        let signature = iter.next()?;
        let key = iter.next()?;
        verify(key, signature)?;
        let key = base64::decode(key).ok()?;
        Uuid::from_slice(key.as_slice()).ok()
    }
}

#[test]
fn test_session_sign() {
    let user_id = Uuid::new_v4();
    let session = Session::new(&user_id);
    let token = session.token();
    assert_eq!(Session::verify(""), None);
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

async fn get_session(value: &str) -> Option<Session> {
    let mut iter = value.split('.');
    let signature = iter.next()?;
    let session_key = iter.next()?;
    verify(session_key, signature)?;
    let bytes = base64::decode(session_key).ok()?;
    let session_key = Uuid::from_slice(&*bytes).ok()?;
    SessionMap::get().get_session(&session_key).await
}

pub struct Unauthenticated;

pub async fn authenticate(req: &hyper::Request<hyper::Body>) -> Result<Session, Unauthenticated> {
    use hyper::header::{HeaderValue, AUTHORIZATION, COOKIE};

    let headers = req.headers();
    let authorization = headers.get(AUTHORIZATION).map(HeaderValue::to_str);

    if let Some(Ok(token)) = authorization {
        if let Some(session) = get_session(token).await {
            return Ok(session);
        }
    }
    let cookie_value = headers.get(COOKIE).and_then(get_cookie).ok_or(Unauthenticated)?;
    get_session(cookie_value).await.ok_or(Unauthenticated)
}

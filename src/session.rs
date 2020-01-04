use crate::api::Error;
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
    pub csrf_token: Uuid,
}

impl Session {
    pub fn new(user_id: &Uuid) -> Session {
        Session {
            key: id(),
            user_id: *user_id,
            csrf_token: Uuid::new_v4(),
        }
    }

    pub fn token(&self) -> String {
        let mut token = base64::encode(self.key.as_bytes());
        let signed = sign(&*token);
        token.push('.');
        token.push_str(&*signed);
        token
    }

    pub fn verify(token: &str) -> Option<Uuid> {
        let mut iter = token.split('.');
        let key = iter.next()?;
        let sign = iter.next()?;
        verify(key, sign)?;
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
    let cookie_pattern = COOKIE_PATTERN.get_or_init(|| Regex::new(r#"\bsession=(.+);"#).unwrap());
    let value = value.to_str().ok()?;
    cookie_pattern.captures(value)?.get(1).map(|m| m.as_str())
}

async fn get_session(value: &str) -> Option<Session> {
    let mut iter = value.split('.');
    let session_id = iter.next()?;
    let sign = iter.next()?;
    verify(session_id, sign)?;
    let bytes = base64::decode(session_id).ok()?;
    let session_id = Uuid::from_slice(&*bytes).ok()?;
    SessionMap::get().get_session(&session_id).await
}

pub async fn authenticate(req: &hyper::Request<hyper::Body>) -> Result<Session, Error> {
    use hyper::header::{AUTHORIZATION, COOKIE};

    let headers = req.headers();
    if let Some(token) = headers.get(AUTHORIZATION) {
        let token = token.to_str().map_err(|_| Error::bad_request())?;
        return get_session(token).await.ok_or_else(Error::unauthorized);
    }
    let cookie_value = headers
        .get(COOKIE)
        .and_then(get_cookie)
        .ok_or_else(Error::unauthorized)?;
    get_session(cookie_value).await.ok_or_else(Error::unauthorized)
}

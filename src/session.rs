use crate::redis;
use crate::utils::{self, id, sign};
use once_cell::sync::OnceCell;
use regex::Regex;
use uuid::Uuid;

pub fn token(session: &Uuid) -> String {
    // [body (base64)].[sign]
    let mut buffer = String::with_capacity(64);
    base64::encode_config_buf(session.as_bytes(), base64::STANDARD, &mut buffer);
    let signature = sign(&*buffer);
    buffer.push('.');
    base64::encode_config_buf(&signature, base64::STANDARD, &mut buffer);
    buffer
}

pub fn token_verify(token: &str) -> Result<Uuid, Unauthenticated> {
    use Unauthenticated::{AuthFailed, Unexpected};
    let mut iter = token.split('.');
    let session = iter.next().ok_or(Unexpected)?;
    let signature = iter.next().ok_or(Unexpected)?;
    utils::verify(session, signature).ok_or(AuthFailed("The session id and signature do not match."))?;
    let session = base64::decode(session).map_err(|_| Unexpected)?;
    Uuid::from_slice(session.as_slice()).map_err(|_| Unexpected)
}

#[test]
fn test_session_sign() {
    let session = id();
    assert!(token_verify("").is_err());
    let session_2 = token_verify(&*token(&session)).unwrap();
    assert_eq!(session, session_2);
}

fn make_key(session: &Uuid) -> Vec<u8> {
    let mut key: Vec<u8> = Vec::with_capacity(64);
    key.extend_from_slice(b"session:");
    key.extend_from_slice(session.as_bytes());
    key.extend_from_slice(b":user_id");
    key
}

pub async fn start(user_id: &Uuid) -> Option<Uuid> {
    let session = id();
    let key = make_key(&session);
    let mut r = redis::get().await;
    r.set(&key, user_id.as_bytes()).await.ok()?;
    Some(session)
}

#[derive(Debug)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
}

fn get_cookie(value: &hyper::header::HeaderValue) -> Option<&str> {
    static COOKIE_PATTERN: OnceCell<Regex> = OnceCell::new();
    let cookie_pattern = COOKIE_PATTERN.get_or_init(|| Regex::new(r#"\bsession=([^;]+)"#).unwrap());
    let value = value.to_str().ok()?;
    cookie_pattern.captures(value)?.get(1).map(|m| m.as_str())
}

async fn get_session(token: &str) -> Result<Session, Unauthenticated> {
    use Unauthenticated::{AuthFailed, Unexpected};

    let id = token_verify(token)?;

    let key = make_key(&id);
    let mut r = redis::get().await;

    let bytes: Vec<u8> = r
        .get(&*key)
        .await
        .map_err(|_| Unexpected)?
        .ok_or(AuthFailed("The session can't be found."))?;

    let user_id: Uuid = Uuid::from_slice(&*bytes).map_err(|_| Unexpected)?;
    Ok(Session { id, user_id })
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

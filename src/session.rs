use crate::cache;
use crate::error::AppError::{self, BadRequest, Unauthenticated};
use crate::error::CacheError;
use crate::utils::{self, sign};
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

pub fn token_verify(token: &str) -> Result<Uuid, AppError> {
    let mut iter = token.split('.');
    let parse_failed = || BadRequest(format!("Failed to parse token"));
    let session = iter.next().ok_or_else(parse_failed)?;
    let signature = iter.next().ok_or_else(parse_failed)?;
    utils::verify(session, signature).ok_or(Unauthenticated)?;
    let session = base64::decode(session).map_err(unexpected!())?;
    Uuid::from_slice(session.as_slice()).map_err(unexpected!())
}

pub async fn revoke_session(id: &Uuid) {
    let key = make_key(id);
    let mut redis = cache::get().await;
    redis.remove(&*key).await.ok();
}

#[test]
fn test_session_sign() {
    let session = utils::id();
    assert!(token_verify("").is_err());
    let session_2 = token_verify(&*token(&session)).unwrap();
    assert_eq!(session, session_2);
}

fn make_key(session: &Uuid) -> Vec<u8> {
    cache::make_key(b"sessions", session, b"user_id")
}

pub async fn start(user_id: &Uuid) -> Result<Uuid, CacheError> {
    let session = utils::id();
    let key = make_key(&session);
    let mut r = cache::get().await;
    r.set(&key, user_id.as_bytes()).await?;
    Ok(session)
}

#[derive(Debug)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
}

pub async fn remove_session(id: Uuid) -> Result<(), CacheError> {
    let mut cache = cache::get().await;
    let key = make_key(&id);
    cache.remove(&*key).await?;
    Ok(())
}

fn get_cookie(value: &hyper::header::HeaderValue) -> Option<&str> {
    static COOKIE_PATTERN: OnceCell<Regex> = OnceCell::new();
    let cookie_pattern = COOKIE_PATTERN.get_or_init(|| Regex::new(r#"\bsession=([^;]+)"#).unwrap());
    let value = value.to_str().ok()?;
    cookie_pattern.captures(value)?.get(1).map(|m| m.as_str())
}

pub async fn authenticate(req: &hyper::Request<hyper::Body>) -> Result<Session, AppError> {
    use hyper::header::{HeaderValue, AUTHORIZATION, COOKIE};

    let headers = req.headers();
    let authorization = headers.get(AUTHORIZATION).map(HeaderValue::to_str);

    let token;
    if let Some(Ok(t)) = authorization {
        token = t;
    } else {
        token = headers.get(COOKIE).and_then(get_cookie).ok_or(Unauthenticated)?;
    }

    let id = token_verify(token)?;

    let key = make_key(&id);
    let mut cache = cache::get().await;
    let bytes: Vec<u8> = cache.get(&*key).await.map_err(unexpected!())?.ok_or(Unauthenticated)?;

    let user_id = Uuid::from_slice(&*bytes).map_err(unexpected!())?;
    Ok(Session { id, user_id })
}

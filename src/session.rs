use crate::cache;
use crate::error::AppError::{self, Unauthenticated};
use crate::error::CacheError;
use crate::utils::{self, sign};
use once_cell::sync::OnceCell;
use regex::Regex;
use uuid::Uuid;
use anyhow::Context;

pub fn token(session: &Uuid) -> String {
    // [body (base64)].[sign]
    let mut buffer = String::with_capacity(64);
    base64::encode_config_buf(session.as_bytes(), base64::STANDARD, &mut buffer);
    let signature = sign(&*buffer);
    buffer.push('.');
    base64::encode_config_buf(&signature, base64::STANDARD, &mut buffer);
    buffer
}

pub fn token_verify(token: &str) -> Result<Uuid, anyhow::Error> {
    let mut iter = token.split('.');
    let parse_failed = || anyhow::anyhow!("Failed to parse token: {}", token);
    let session = iter.next().ok_or_else(parse_failed)?;
    let signature = iter.next().ok_or_else(parse_failed)?;
    utils::verify(session, signature)?;
    let session = base64::decode(session).context("Failed to decode base64 in session.")?;
    Uuid::from_slice(session.as_slice()).context("Failed to convert session bytes data to UUID.")
}

pub async fn revoke_session(id: &Uuid) -> Result<(), CacheError> {
    let key = make_key(id);
    cache::conn().await.remove(&*key).await
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
    cache::conn().await.set(&key, user_id.as_bytes()).await?;
    Ok(session)
}

#[derive(Debug)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
}

pub async fn remove_session(id: Uuid) -> Result<(), CacheError> {
    let key = make_key(&id);
    cache::conn().await.remove(&*key).await?;
    Ok(())
}

fn parse_cookie(value: &hyper::header::HeaderValue) -> Result<&str, anyhow::Error> {
    static COOKIE_PATTERN: OnceCell<Regex> = OnceCell::new();
    let cookie_pattern = COOKIE_PATTERN.get_or_init(|| Regex::new(r#"\bsession=([^;]+)"#).unwrap());
    let value = value.to_str().with_context(|| format!("Failed to convert {:?} to string.", value))?;
    let failed = || anyhow::anyhow!("Failed to parse cookie: {}", value);
    let capture = cookie_pattern
        .captures(value)
        .ok_or_else(failed)?;
    capture
        .get(1)
        .map(|m| m.as_str())
        .ok_or_else(failed)
}

pub async fn authenticate(req: &hyper::Request<hyper::Body>) -> Result<Session, AppError> {
    use hyper::header::{HeaderValue, AUTHORIZATION, COOKIE};

    let headers = req.headers();
    let authorization = headers.get(AUTHORIZATION).map(HeaderValue::to_str);

    let token = if let Some(Ok(t)) = authorization {
        t
    } else {
        let cookie = headers
            .get(COOKIE)
            .ok_or(Unauthenticated(format!("There is no cookie in header")))?;
        let token = parse_cookie(cookie);

        token.map_err(|err| {
            log::warn!("Failed to parse cookie: {}", err);
            Unauthenticated(format!("Invalid cookie"))
        })?
    };

    let id = match token_verify(token) {
        Err(err) => {
            log::warn!("{}", err);
            return Err(AppError::Unauthenticated(format!("Invalid session")))
        }
        Ok(id) => id,
    };

    let key = make_key(&id);
    let bytes: Vec<u8> = cache::conn()
        .await
        .get(&*key)
        .await
        .map_err(error_unexpected!())?
        .ok_or_else(|| {
            log::warn!("Session {} not found, token: {}", id, token);
            Unauthenticated(format!("Session not found"))
        })?;

    let user_id = Uuid::from_slice(&*bytes).map_err(error_unexpected!())?;
    Ok(Session { id, user_id })
}

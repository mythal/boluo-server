use crate::api;
use crate::session::{self, Session, Unauthenticated};
use crate::utils::{now_unix_duration, sign, verify};
use hyper::header::{HeaderName, AUTHORIZATION};
use hyper::{Body, Request};
use once_cell::sync::OnceCell;
use regex::Regex;
use uuid::Uuid;

// csrf-token:[signature].[session key(base 64)].[timestamp]

pub async fn authenticate(req: &Request<Body>) -> Result<Session, Unauthenticated> {
    use hyper::Method;
    let session = session::authenticate(req).await?;
    let method = req.method();
    if method == Method::GET || method == Method::HEAD || req.headers().contains_key(AUTHORIZATION) {
        return Ok(session);
    }
    let headers = req.headers();
    let token_header = HeaderName::from_static("csrf-token");
    let token = headers
        .get(token_header)
        .and_then(|header_value| header_value.to_str().ok())
        .ok_or(Unauthenticated)?;

    static PATTERN: OnceCell<Regex> = OnceCell::new();
    let csrf_pattern = PATTERN.get_or_init(|| Regex::new(r#"^([^.]+)\.(([^.]+)\.([^.]+))"#).unwrap());

    let captured = csrf_pattern.captures(token).ok_or(Unauthenticated)?;

    let signature = captured.get(1).ok_or(Unauthenticated)?.as_str();
    let body = captured.get(2).ok_or(Unauthenticated)?.as_str();
    verify(body, signature).ok_or(Unauthenticated)?;

    let session_key = captured
        .get(3)
        .map(|m| m.as_str()) // get base64 encoded string.
        .and_then(|s| base64::decode(s).ok()) // decode.
        .and_then(|bytes: Vec<u8>| Uuid::from_slice(&*bytes).ok()) // convert bytes to UUID.
        .ok_or(Unauthenticated)?;
    if session_key != session.key {
        return Err(Unauthenticated);
    }

    let timestamp: u64 = captured
        .get(4)
        .map(|m| m.as_str())
        .and_then(|s| s.parse().ok())
        .ok_or(Unauthenticated)?;
    let now = now_unix_duration().as_secs();
    if timestamp < now {
        return Err(Unauthenticated);
    }
    Ok(session)
}

pub fn generate_csrf_token(session_key: &Uuid) -> String {
    let expire_sec = 60 * 60 * 3;
    let timestamp: u64 = now_unix_duration().as_secs() + expire_sec;

    let mut body = base64::encode(session_key.as_bytes());
    body.push_str(&*format!(".{}", timestamp));
    format!("{}.{}", sign(&*body), body)
}

pub async fn get_csrf_token(req: Request<Body>) -> api::Result {
    let session = session::authenticate(&req).await?;
    let token = generate_csrf_token(&session.key);
    dbg!(token.len());
    api::Return::new(&token).build()
}

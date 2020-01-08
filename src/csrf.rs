use crate::api;
use crate::session::{self, Session, Unauthenticated};
use crate::utils::{now_unix_duration, sign, verify};
use hyper::header::{HeaderName, AUTHORIZATION};
use hyper::{Body, Request};
use uuid::Uuid;

// csrf-token:[session key(base 64)].[timestamp].[signature]

pub async fn authenticate(req: &Request<Body>) -> Result<Session, Unauthenticated> {
    use crate::session::Unauthenticated::{AuthFailed, ParseFailed, Unexpected};
    use hyper::Method;
    let session = session::authenticate(req).await?;
    let method = req.method();
    if method == Method::GET || method == Method::HEAD || req.headers().contains_key(AUTHORIZATION) {
        return Ok(session);
    }
    let token = req
        .headers()
        .get(HeaderName::from_static("csrf-token"))
        .and_then(|header_value| header_value.to_str().ok())
        .ok_or(ParseFailed("Can't get csrf-token in the headers."))?;

    let (body, signature) = token.rfind('.').map(|pos| token.split_at(pos)).ok_or(Unexpected)?;
    verify(body, &signature[1..]).ok_or(AuthFailed("Mismatched CSRF token and signature."))?;

    let mut parts = body.split('.');

    let session_key = parts
        .next()
        .and_then(|s| base64::decode(s).ok()) // decode.
        .and_then(|bytes: Vec<u8>| Uuid::from_slice(&*bytes).ok()) // convert bytes to UUID.
        .ok_or(ParseFailed("Can't retrieve session key"))?;
    if session_key != session.key {
        return Err(AuthFailed("Invalid session key"));
    }

    let timestamp: u64 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or(ParseFailed("Can't retrieve timestamp"))?;
    let now = now_unix_duration().as_secs();
    if timestamp < now {
        return Err(AuthFailed("Timeout"));
    }
    Ok(session)
}

pub fn generate_csrf_token(session_key: &Uuid) -> String {
    let expire_sec = 60 * 60 * 3;
    let timestamp: u64 = now_unix_duration().as_secs() + expire_sec;
    let mut buffer = String::with_capacity(128);
    base64::encode_config_buf(session_key.as_bytes(), base64::STANDARD, &mut buffer);
    buffer.push('.');
    buffer.push_str(&*timestamp.to_string());
    let signature = sign(&*buffer);
    buffer.push('.');
    base64::encode_config_buf(&signature, base64::STANDARD, &mut buffer);
    buffer
}

pub async fn get_csrf_token(req: Request<Body>) -> api::Result {
    let session = session::authenticate(&req).await?;
    let token = generate_csrf_token(&session.key);
    api::Return::new(&token).build()
}

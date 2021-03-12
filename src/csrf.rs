use crate::error::AppError::{self, BadRequest, Unauthenticated};
use crate::session::{self, Session};
use crate::utils::{now_unix_duration, sign, verify};
use hyper::header::{HeaderName, AUTHORIZATION};
use hyper::{Body, Request};
use uuid::Uuid;

// csrf-token:[session key(base 64)].[timestamp].[signature]

pub async fn authenticate(req: &Request<Body>) -> Result<Session, AppError> {
    use hyper::Method;
    let session = session::authenticate(req).await?;
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

pub async fn get_csrf_token(req: Request<Body>) -> Result<String, AppError> {
    let session_id = if let Ok(session) = session::authenticate(&req).await {
        session.id
    } else {
        Uuid::nil()
    };

    Ok(generate_csrf_token(&session_id))
}

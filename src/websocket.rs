use hyper::header::{HeaderValue, HeaderMap, SEC_WEBSOCKET_KEY, UPGRADE, CONNECTION};
use hyper::upgrade::Upgraded;
use std::future::Future;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Error;
use crate::error::AppError;
use crate::utils::sha1;
use crate::common::{Request, Response};
use hyper::Body;

pub fn check_websocket_header(headers: &HeaderMap) -> Result<HeaderValue, AppError> {
    let upgrade = headers.get(UPGRADE)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::BadRequest(String::new()))?;
    if upgrade.trim() != "websocket" {
        return Err(AppError::BadRequest(String::new()));
    }
    let connection = headers.get(CONNECTION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::BadRequest(String::new()))?;
    if connection.trim() != "Upgrade" {
        return Err(AppError::BadRequest(String::new()));
    }
    let mut key = headers.get(SEC_WEBSOCKET_KEY)
        .and_then(|key| key.to_str().ok())
        .ok_or(AppError::BadRequest("Failed to read ws key from headers".to_string()))?
        .to_string();
    key.push_str("258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let accept = base64::encode(sha1(key.as_bytes()).as_ref());
    HeaderValue::from_str(&*accept).map_err(unexpected!())
}


pub fn establish_web_socket<H, F>(req: Request, handler: H) -> Result<Response, AppError>
where
    H: FnOnce(WebSocketStream<Upgraded>) -> F,
    H: Send + 'static,
    F: Future<Output=()> + Send,
{
    use tokio_tungstenite::tungstenite::protocol::Role;
    use hyper::{header, StatusCode};
    let accept = check_websocket_header(req.headers())?;
    tokio::spawn(async {
        match req.into_body().on_upgrade().await {
            Ok(upgraded) => {
                let ws_stream = tokio_tungstenite::WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;
                log::debug!("WebSocket connection established");
                handler(ws_stream).await;
            }
            Err(e) => {
                log::error!("Failed to upgrade connection: {}", e);
            }
        }
        log::debug!("WebSocket disconnected");
    });
    hyper::Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "Upgrade")
        .header(header::SEC_WEBSOCKET_ACCEPT, accept)
        .body(Body::empty())
        .map_err(unexpected!())
}

pub fn log_error(e: &Error) {
    match e {
        Error::ConnectionClosed => (),
        Error::AlreadyClosed => log::info!("{}", e),
        e => log::warn!("{}", e),
    }
}

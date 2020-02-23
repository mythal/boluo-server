use super::api::EventQuery;
use super::Event;
use crate::common::{parse_query, Response, missing, ok_response, IdQuery};
use crate::error::AppError;
use std::time::Duration;
use hyper::{Body, Request};


use futures::{StreamExt, SinkExt};
use futures::stream::SplitSink;
use crate::websocket::{establish_web_socket};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite;
use hyper::upgrade::Upgraded;
use uuid::Uuid;
use crate::events::EventBody;
use std::sync::Arc;

type Sender = SplitSink<WebSocketStream<Upgraded>, tungstenite::Message>;


async fn events(req: Request<Body>) -> Result<Vec<Event>, AppError> {
    let EventQuery { mailbox, after } = parse_query(req.uri())?;
    Event::get_from_cache(&mailbox, after).await.map_err(Into::into)
}

async fn push(mailbox: Uuid, tx: &mut Sender) -> Result<(), anyhow::Error> {
    use tokio::time::interval;
    
    use tokio_tungstenite::tungstenite::Error::{ConnectionClosed, AlreadyClosed};
    let ping = tungstenite::Message::Ping(Vec::new());
    let refresh = Arc::new(Event::new(mailbox, EventBody::Refresh));
    let mut interval = interval(Duration::from_secs(30));
    loop {
        let wait = Event::wait(mailbox);
        let message = tokio::select! {
            _ = interval.next() => ping.clone(),
            event = wait => {
                let event = event.unwrap_or_else(|_| refresh.clone());
                tungstenite::Message::Text(serde_json::to_string(&*event)?)
            },
        };
        match tx.send(message).await {
            Ok(_) => (),
            Err(ConnectionClosed) | Err(AlreadyClosed) => break,
            e => e?,
        }
    }
    Ok(())
}

async fn connect(req: Request<Body>) -> Result<Response, AppError> {
    let IdQuery { id } = parse_query(req.uri())?;
    establish_web_socket(req, move |ws_stream| async move {
        let (mut tx, mut rx) = ws_stream.split();
        tokio::spawn(async move {
            if let Err(e) = push(id, &mut tx).await {
                log::warn!("push {}: {}", id, e);
            }
            tx.close().await.ok();
            log::debug!("{} write stream close", id);
        });
        loop {
            let close_timeout = tokio::time::delay_for(Duration::from_secs(60));
            let message = tokio::select! {
                _ = close_timeout => break,
                message = rx.next() => message,
            };
            match message {
                None | Some(Ok(tungstenite::Message::Close(_))) => break,
                Some(Ok(tungstenite::Message::Pong(_))) => (),
                Some(Ok(message)) => {
                },
                Some(Err(e)) => {
                    log::warn!("read {}: {}", id, e);
                    break;
                },
            }
        }
        while let Some(message) = rx.next().await {
            match message {
                Ok(tungstenite::Message::Close(_)) => break,
                Ok(tungstenite::Message::Pong(_)) => {
                    log::debug!("pong");
                },
                Ok(_message) => {
                }
                Err(e) => {
                    log::warn!("read {}: {}", id, e);
                    break;
                }
            }
        }
        log::debug!("read stream close");
    })
}

pub async fn router(req: Request<Body>, path: &str) -> Result<Response, AppError> {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/events", Method::GET) => events(req).await.map(ok_response),
        ("/connect", Method::GET) => connect(req).await,
        _ => missing(),
    }
}

use super::events::EventQuery;
use super::Event;
use crate::common::{parse_query, Response, missing};
use crate::error::AppError;
use std::time::Duration;
use anyhow::anyhow;
use hyper::{Body, Request};
use futures::{StreamExt, SinkExt, TryStreamExt};
use futures::stream::SplitSink;
use crate::websocket::{establish_web_socket, WsMessage, WsError};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite;
use hyper::upgrade::Upgraded;
use uuid::Uuid;
use crate::events::events::ClientEvent;
use crate::csrf::authenticate;
use crate::events::context::{get_receiver, get_preview_cache};

type Sender = SplitSink<WebSocketStream<Upgraded>, tungstenite::Message>;

async fn push(mailbox: Uuid, outgoing: &mut Sender, after: i64) -> Result<(), anyhow::Error> {
    use tokio::time::interval;
    use tokio::sync::broadcast::RecvError;
    use futures::channel::mpsc::channel;
    use tokio_tungstenite::tungstenite::Error::{ConnectionClosed, AlreadyClosed};
    let (tx, mut rx) = channel::<WsMessage>(32);
    let send_message = async move {
        while let Some(message) = rx.next().await {
            match outgoing.send(message).await {
                Ok(_) => (),
                Err(ConnectionClosed) | Err(AlreadyClosed) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    };
    let push = async {
        let mut tx = tx.clone();
        let mut receiver = get_receiver(&mailbox).await;
        let events = Event::get_from_cache(&mailbox, after).await;
        Event::push_members(mailbox);
        let previews: Vec<String> = {
            let preview_cache = get_preview_cache();
            let channel_map = preview_cache.lock().await;
            channel_map
                .get(&mailbox)
                .map(|user_map| user_map.values().map(|event| event.encoded.clone()).collect())
                .unwrap_or(Vec::new())
        };
        match events {
            Ok(events) => {
                for e in events.into_iter().chain(previews.into_iter()) {
                    tx.send(WsMessage::Text(e)).await.ok();
                }
            }
            Err(e) => Err(anyhow!("failed to get events from cache: {}", e))?,
        }
        loop {
            let message = match receiver.recv().await {
                Ok(event) => WsMessage::Text(event.encoded),
                Err(RecvError::Lagged(lagged)) => {
                    log::warn!("lagged {} at {}", lagged, mailbox);
                    continue;
                },
                Err(RecvError::Closed) => return Err(anyhow!("broadcast ({}) is closed.", mailbox)),
            };
            if let Err(_) = tx.send(message).await {
                break
            }
        }
        Ok(())
    };
    let ping = interval(Duration::from_secs(30))
        .for_each(|_| {
            async {
                tx.clone().send(WsMessage::Ping(Vec::new())).await.ok();
            }
        });

    tokio::select! {
        r = send_message => { r?; },
        _ = ping => {},
        r = push => { r? },
    }
    Event::get_from_cache(&mailbox, after).await?;

    Ok(())
}

async fn receive_message(user_id: Option<Uuid>, message: String) -> Result<(), anyhow::Error> {
    let event: ClientEvent = serde_json::from_str(&*message)?;
    match event {
        ClientEvent::Preview { preview } => {
            let user_id = user_id.ok_or(AppError::Unauthenticated)?;
            preview.broadcast(user_id).await?;
        },
        ClientEvent::Heartbeat { mailbox } => {
            if let Some(user_id) = user_id {
                Event::heartbeat(mailbox, user_id);
            }
        },
    }
    Ok(())
}

async fn connect(req: Request<Body>) -> Result<Response, AppError> {
    use tokio::stream::StreamExt as _;
    use futures::future;
    let user_id = authenticate(&req).await.ok().map(|session| session.user_id);

    let EventQuery { mailbox, after } = parse_query(req.uri())?;
    establish_web_socket(req, move |ws_stream| async move {
        let (mut outgoing, incoming) = ws_stream.split();
        let handle_push = async move {
            if let Err(e) = push(mailbox, &mut outgoing, after).await {
                log::warn!("Failed to push event: {}", e);
            }
            outgoing.close().await.ok();
        };
        let handle_messages = incoming
            .timeout(Duration::from_secs(40))
            .map_err(|_| WsError::AlreadyClosed)
            .and_then(future::ready)
            .try_for_each(|message: WsMessage| {
                async move {
                    if let WsMessage::Text(message) = message {
                        if let Err(e) = receive_message(user_id, message).await {
                            log::warn!("Failed to send event: {}", e);
                        }
                    }
                    Ok(())
                }
            });
        futures::pin_mut!(handle_push);
        futures::pin_mut!(handle_messages);
        future::select(handle_push, handle_messages).await;
        log::debug!("WebSocket connection close");
    })
}

pub async fn router(req: Request<Body>, path: &str) -> Result<Response, AppError> {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/connect", Method::GET) => connect(req).await,
        _ => missing(),
    }
}

use super::api::EventQuery;
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
use crate::events::api::ClientEvent;
use crate::csrf::authenticate;
use crate::database;
use crate::channels::ChannelMember;
use crate::messages::api::NewPreview;
use crate::messages::Preview;
use crate::events::get_receiver;

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
        let (mut receiver, events) = futures::future::join(
            get_receiver(&mailbox),
            Event::get_from_cache(&mailbox, after)
        ).await;
        match events {
            Ok(events) =>
                for e in events {
                    tx.send(WsMessage::Text(e)).await.ok();
                },
            Err(e) => Err(anyhow!("failed to get events from cache: {}", e))?,
        }
        loop {
            let message = match receiver.recv().await {
                Ok(event_encoded) => WsMessage::Text(event_encoded),
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
        ClientEvent::Preview(
            NewPreview {
                id,
                channel_id,
                name,
                media_id,
                in_game,
                is_action,
                text,
                entities,
                whisper_to_users,
                start
            }
        ) => {
            let user_id = user_id.ok_or(AppError::Unauthenticated)?;
            let mut conn = database::get().await;
            let db = &mut *conn;
            let member = ChannelMember::get(db, &user_id, &channel_id)
                .await?
                .ok_or(AppError::NoPermission)?;
            Event::message_preview(Preview {
                id,
                sender_id: user_id,
                channel_id,
                parent_message_id: None,
                name,
                media_id,
                in_game,
                is_action,
                text,
                whisper_to_users,
                entities,
                start,
                is_master: member.is_master,
            })
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

use super::events::EventQuery;
use super::Event;
use crate::csrf::authenticate;
use crate::error::AppError;
use crate::events::context::get_mailbox_broadcast_rx;
use crate::events::events::{ClientEvent, MailBoxType};
use crate::interface::{missing, parse_query, Response};
use crate::websocket::{establish_web_socket, WsError, WsMessage};
use anyhow::anyhow;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt, TryStreamExt};
use hyper::upgrade::Upgraded;
use hyper::{Body, Request};
use std::time::Duration;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;
use crate::database;
use crate::spaces::{Space, SpaceMember};
use crate::channels::{Channel, ChannelMember};
use crate::database::Querist;

type Sender = SplitSink<WebSocketStream<Upgraded>, tungstenite::Message>;


async fn check_space_perms<T: Querist>(db: &mut T, space: &Space, user_id: Option<Uuid>) -> Result<(), AppError> {
    if !space.allow_spectator {
        if let Some(user_id) = user_id {
            let space_member = SpaceMember::get(db, &user_id, &space.id).await?;
            if space_member.is_none() {
                return Err(AppError::NoPermission);
            }
        } else {
            return Err(AppError::NoPermission);
        }
    }
    Ok(())
}

async fn push_events(mailbox: Uuid, _mailbox_type: MailBoxType, outgoing: &mut Sender, after: i64) -> Result<(), anyhow::Error> {
    use futures::channel::mpsc::channel;
    use tokio::sync::broadcast::RecvError;
    use tokio::time::interval;
    use tokio_tungstenite::tungstenite::Error::{AlreadyClosed, ConnectionClosed};
    let (tx, mut rx) = channel::<WsMessage>(32);
    let message_sender = async move {
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
        let mut mailbox_rx = get_mailbox_broadcast_rx(&mailbox).await;

        let cached_events = Event::get_from_cache(&mailbox, after).await;
        for e in cached_events.into_iter() {
            tx.send(WsMessage::Text(e)).await.ok();
        }

        loop {
            let message = match mailbox_rx.recv().await {
                Ok(event) => WsMessage::Text(event.encoded.clone()),
                Err(RecvError::Lagged(lagged)) => {
                    log::warn!("lagged {} at {}", lagged, mailbox);
                    continue;
                }
                Err(RecvError::Closed) => return Err(anyhow!("broadcast ({}) is closed.", mailbox)),
            };
            if tx.send(message).await.is_err() {
                break;
            }
        }
        Ok(())
    };

    let ping = interval(Duration::from_secs(30)).for_each(|_| async {
        tx.clone().send(WsMessage::Ping(Vec::new())).await.ok();
    });

    tokio::select! {
        r = message_sender => { r? },
        _ = ping => {},
        r = push => { r? },
    }

    Ok(())
}

async fn handle_client_event(
    mailbox: Uuid,
    mailbox_type: MailBoxType,
    user_id: Option<Uuid>,
    message: String,
) -> Result<(), anyhow::Error> {
    let event: ClientEvent = serde_json::from_str(&*message)?;
    match event {
        ClientEvent::Preview { preview } => {
            let user_id = user_id.ok_or(AppError::Unauthenticated)?;
            preview.broadcast(mailbox, mailbox_type, user_id).await?;
        }
        ClientEvent::Heartbeat => {
            if let Some(user_id) = user_id {
                Event::heartbeat(mailbox, user_id).await;
            }
        }
    }
    Ok(())
}

async fn connect(req: Request<Body>) -> Result<Response, AppError> {
    use futures::future;
    use tokio::stream::StreamExt as _;
    let user_id = authenticate(&req).await.ok().map(|session| session.user_id);

    let EventQuery {
        mailbox,
        mailbox_type,
        after,
    } = parse_query(req.uri())?;

    let mut conn = database::get().await?;
    let db = &mut *conn;
    match mailbox_type {
        MailBoxType::Space => {
            let space = Space::get_by_id(db, &mailbox)
                .await?
                .ok_or_else(|| AppError::NotFound("space"))?;
            check_space_perms(db, &space, user_id).await?;
        },
        MailBoxType::Channel => {
            let channel = Channel::get_by_id(db, &mailbox)
                .await?
                .ok_or_else(|| AppError::NotFound("channel"))?;
            let space = Space::get_by_id(db, &channel.space_id)
                .await?
                .ok_or_else(|| AppError::NotFound("space"))?;
            check_space_perms(db, &space, user_id).await?;
            if !channel.is_public {
                let user_id = user_id.ok_or(AppError::Unauthenticated)?;
                ChannelMember::get(db, &user_id, &channel.id)
                    .await?
                    .ok_or(AppError::Unauthenticated)?;
            }
        },
    }
    establish_web_socket(req, move |ws_stream| async move {
        let (mut outgoing, incoming) = ws_stream.split();

        let server_push_events = async move {
            if let Err(e) = push_events(mailbox, mailbox_type, &mut outgoing, after).await {
                log::warn!("Failed to push events: {}", e);
            }
            outgoing.close().await.ok();
        };

        let receive_client_events = incoming
            .timeout(Duration::from_secs(40))
            .map_err(|_| WsError::AlreadyClosed)
            .and_then(future::ready)
            .try_for_each(|message: WsMessage| async move {
                if let WsMessage::Text(message) = message {
                    if let Err(e) = handle_client_event(mailbox, mailbox_type, user_id, message).await {
                        log::warn!("Failed to send event: {}", e);
                    }
                }
                Ok(())
            });
        futures::pin_mut!(server_push_events);
        futures::pin_mut!(receive_client_events);
        future::select(server_push_events, receive_client_events).await;
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

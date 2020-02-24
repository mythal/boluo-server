use super::api::EventQuery;
use super::Event;
use crate::common::{parse_query, Response, missing, ok_response, IdQuery};
use crate::error::AppError;
use std::time::Duration;
use hyper::{Body, Request};
use futures::{StreamExt, SinkExt, TryStreamExt};
use futures::stream::SplitSink;
use crate::websocket::{establish_web_socket, WsMessage, WsError};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite;
use hyper::upgrade::Upgraded;
use uuid::Uuid;
use crate::events::EventBody;
use std::sync::Arc;
use crate::events::api::ClientEvent;
use crate::csrf::authenticate;
use crate::database;
use crate::channels::ChannelMember;
use crate::messages::api::NewPreview;
use crate::messages::Preview;

type Sender = SplitSink<WebSocketStream<Upgraded>, tungstenite::Message>;


async fn events(req: Request<Body>) -> Result<Vec<Event>, AppError> {
    let EventQuery { mailbox, after } = parse_query(req.uri())?;
    Event::get_from_cache(&mailbox, after).await.map_err(Into::into)
}

async fn push(mailbox: Uuid, outgoing: &mut Sender) -> Result<(), anyhow::Error> {
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
        match outgoing.send(message).await {
            Ok(_) => (),
            Err(ConnectionClosed) | Err(AlreadyClosed) => break,
            e => e?,
        }
    }
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

    let IdQuery { id } = parse_query(req.uri())?;
    establish_web_socket(req, move |ws_stream| async move {
        let (mut outgoing, incoming) = ws_stream.split();
        let handle_push = async move {
            if let Err(e) = push(id, &mut outgoing).await {
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
        ("/events", Method::GET) => events(req).await.map(ok_response),
        ("/connect", Method::GET) => connect(req).await,
        _ => missing(),
    }
}

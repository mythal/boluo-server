use super::api::{Edit, NewMessage};
use super::Message;
use crate::channels::{fire, ChannelMember, Event};
use crate::csrf::authenticate;
use crate::database::Querist;
use crate::error::AppError;
use crate::messages::Preview;
use crate::{api, database};
use hyper::{Body, Request};
use uuid::Uuid;

async fn channel_member<T: Querist>(db: &mut T, user_id: &Uuid, channel_id: &Uuid) -> Result<ChannelMember, AppError> {
    let channel_member = ChannelMember::get(db, &user_id, &channel_id)
        .await
        .ok_or(AppError::Unauthenticated)?;
    Ok(channel_member)
}

async fn send(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let NewMessage {
        message_id,
        channel_id,
        name,
        text,
        entities,
        in_game,
        is_action,
    } = api::parse_body(req).await?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let member = channel_member(db, &session.user_id, &channel_id).await?;
    let name = name.unwrap_or(member.character_name);
    let message = Message::create(
        db,
        message_id.as_ref(),
        &channel_id,
        &session.user_id,
        &*name,
        &*text,
        &entities,
        in_game,
        is_action,
    )
    .await?;
    let result = api::Return::new(&message).build();
    fire(&channel_id, Event::new_message(message));
    result
}

async fn edit(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let Edit {
        message_id,
        name,
        text,
        entities,
        in_game,
        is_action,
    } = api::parse_body(req).await?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let (message, _) = Message::get_with_space_member(db, &message_id)
        .await
        .map_err(|_| AppError::Unauthenticated)?;
    if message.sender_id == session.user_id {
        return Err(AppError::Unauthenticated);
    }

    let text = text.as_ref().map(String::as_str);
    let name = name.as_ref().map(String::as_str);
    let message = Message::edit(db, name, &message_id, text, &entities, in_game, is_action).await?;
    let result = api::Return::new(&message).build();
    let channel_id = message.channel_id.clone();
    fire(&channel_id, Event::message_edited(channel_id, message));
    result
}

async fn query(req: Request<Body>) -> api::Result {
    let api::IdQuery { id } = api::parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let message = Message::get(db, &id).await?;
    api::Return::new(&message).build()
}

async fn delete(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let api::IdQuery { id } = api::parse_body(req).await?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let (message, space_member) = Message::get_with_space_member(db, &id).await?;
    let space_member = space_member.ok_or(AppError::Unauthenticated)?;
    if !(message.sender_id == session.user_id || space_member.is_admin) {
        return Err(AppError::Unauthenticated);
    }
    Message::delete(db, &id).await?;
    let channel_id = message.channel_id.clone();
    let event = Event::message_deleted(channel_id, message.id.clone());
    fire(&message.channel_id, event);
    api::Return::new(true).build()
}

async fn send_preview(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let preview: Preview = api::parse_body(req).await?;

    if preview.sender_id != session.user_id {
        log::warn!("The user {} attempts to forge preview message.", session.user_id);
        return Err(AppError::BadRequest(format!("You are forging message")));
    }

    let mut conn = database::get().await;
    let db = &mut *conn;
    let channel_id = preview.channel_id.clone();

    ChannelMember::get_with_space_member(db, &channel_id)
        .await?
        .ok_or(AppError::Unauthenticated)?;
    fire(&channel_id, Event::message_preview(channel_id.clone(), preview));
    api::Return::new(true).build()
}

pub async fn router(req: Request<Body>, path: &str) -> api::Result {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/query", Method::GET) => query(req).await,
        ("/send", Method::POST) => send(req).await,
        ("/delete", Method::DELETE) => delete(req).await,
        ("/edit", Method::POST) => edit(req).await,
        ("/preview", Method::POST) => send_preview(req).await,
        _ => Err(AppError::missing()),
    }
}

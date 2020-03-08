use super::api::{Edit, NewMessage};
use super::Message;
use crate::common::{parse_query, Response, missing, ok_response};
use crate::channels::{Channel, ChannelMember};
use crate::csrf::authenticate;
use crate::error::AppError;
use crate::events::Event;
use crate::spaces::SpaceMember;
use crate::{common, database};
use hyper::{Body, Request};
use crate::messages::api::{ByChannel};

async fn send(req: Request<Body>) -> Result<Message, AppError> {
    let session = authenticate(&req).await?;
    let NewMessage {
        message_id,
        channel_id,
        name,
        text,
        entities,
        in_game,
        is_action,
        order_date,
        media_id,
    } = common::parse_body(req).await?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let channel_member = ChannelMember::get(db, &session.user_id, &channel_id)
        .await?
        .ok_or(AppError::NoPermission)?;
    let message = Message::create(
        db,
        message_id.as_ref(),
        &channel_id,
        &session.user_id,
        &*channel_member.character_name,
        &*name,
        &*text,
        entities,
        in_game,
        is_action,
        channel_member.is_master,
        None,
        media_id,
        order_date,
    )
    .await?;
    Event::new_message(message.clone());
    Ok(message)
}

async fn edit(req: Request<Body>) -> Result<Message, AppError> {
    let session = authenticate(&req).await?;
    let Edit {
        message_id,
        name,
        text,
        entities,
        in_game,
        is_action,
    } = common::parse_body(req).await?;
    let mut db = database::get().await;
    let mut trans = db.transaction().await?;
    let db = &mut trans;
    let message = Message::get(db, &message_id, Some(&session.user_id))
        .await?
        .ok_or(AppError::NotFound("messages"))?;
    ChannelMember::get(db, &session.user_id, &message.channel_id)
        .await?
        .ok_or(AppError::NoPermission)?;
    if message.sender_id != session.user_id {
        return Err(AppError::NoPermission);
    }
    let text = text.as_ref().map(String::as_str);
    let name = name.as_ref().map(String::as_str);
    let message = Message::edit(db, name, &message_id, text, entities, in_game, is_action, None)
        .await?
        .ok_or_else(|| unexpected!("The message had been delete."))?;
    trans.commit().await?;
    Event::message_edited(message.clone());
    Ok(message)
}

async fn query(req: Request<Body>) -> Result<Message, AppError> {
    let common::IdQuery { id } = common::parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let user_id = authenticate(&req).await.ok().map(|session| session.user_id);
    Message::get(db, &id, user_id.as_ref()).await?.ok_or(AppError::NotFound("message"))
}

async fn delete(req: Request<Body>) -> Result<Message, AppError> {
    let session = authenticate(&req).await?;
    let common::IdQuery { id } = common::parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let message = Message::get(db, &id, None)
        .await?
        .ok_or(AppError::NotFound("messages"))?;
    let space_member = SpaceMember::get_by_channel(db, &session.user_id, &message.channel_id)
        .await?
        .ok_or(AppError::NoPermission)?;
    if !space_member.is_admin {
        return Err(AppError::NoPermission);
    }
    Message::delete(db, &id).await?;
    Event::message_deleted(message.channel_id, message.id);
    Ok(message)
}

async fn toggle_fold(req: Request<Body>) -> Result<Message, AppError> {
    let session = authenticate(&req).await?;
    let common::IdQuery { id } = common::parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let message = Message::get(db, &id, None)
        .await?
        .ok_or(AppError::NotFound("messages"))?;
    let space_member = SpaceMember::get_by_channel(db, &session.user_id, &message.channel_id)
        .await?
        .ok_or(AppError::NoPermission)?;
    if message.sender_id != session.user_id && !space_member.is_admin {
        return Err(AppError::NoPermission);
    }
    let folded = Some(!message.folded);
    let message = Message::edit(db, None, &message.id, None, None, None, None, folded)
        .await?
        .ok_or_else(|| unexpected!("message not found"))?;
    Event::message_edited(message.clone());
    Ok(message)
}

async fn by_channel(req: Request<Body>) -> Result<Vec<Message>, AppError> {
    let ByChannel { channel_id, limit, before } = parse_query(req.uri())?;

    let mut db = database::get().await;
    let db = &mut *db;

    Channel::get_by_id(db, &channel_id)
        .await?
        .ok_or(AppError::NotFound("channels"))?;
    let limit = limit.unwrap_or(128);
    Message::get_by_channel(db, &channel_id, before, limit).await.map_err(Into::into)
}

pub async fn router(req: Request<Body>, path: &str) -> Result<Response, AppError> {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/query", Method::GET) => query(req).await.map(ok_response),
        ("/by_channel", Method::GET) => by_channel(req).await.map(ok_response),
        ("/send", Method::POST) => send(req).await.map(ok_response),
        ("/toggle_fold", Method::POST) => toggle_fold(req).await.map(ok_response),
        ("/delete", Method::POST) => delete(req).await.map(ok_response),
        _ => missing(),
    }
}

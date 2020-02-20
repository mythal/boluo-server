use super::api::{Edit, NewMessage};
use super::Message;
use crate::api::parse_query;
use crate::channels::{Channel, ChannelMember};
use crate::csrf::authenticate;
use crate::error::AppError;
use crate::events::Event;
use crate::messages::Preview;
use crate::spaces::SpaceMember;
use crate::{api, database};
use hyper::{Body, Request};
use crate::messages::api::{ByChannel, NewPreview};

async fn send(req: Request<Body>) -> api::AppResult {
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
    )
    .await?;
    let result = api::Return::new(&message).build();
    Event::new_message(message);
    result
}

async fn edit(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let Edit {
        message_id,
        name,
        text,
        entities,
        in_game,
        is_action,
    } = api::parse_body(req).await?;
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
    let message = Message::edit(db, name, &message_id, text, entities, in_game, is_action)
        .await?
        .ok_or_else(|| unexpected!("The message had been delete."))?;
    trans.commit().await?;
    let result = api::Return::new(&message).build();
    Event::message_edited(message);
    result
}

async fn query(req: Request<Body>) -> api::AppResult {
    let api::IdQuery { id } = api::parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let user_id = authenticate(&req).await.ok().map(|session| session.user_id);
    let message = Message::get(db, &id, user_id.as_ref()).await?;
    api::Return::new(&message).build()
}

async fn delete(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let api::IdQuery { id } = api::parse_body(req).await?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let message = Message::get(db, &id, None)
        .await?
        .ok_or(AppError::NotFound("messages"))?;
    let space_member = SpaceMember::get_by_channel(db, &session.id, &message.channel_id)
        .await?
        .ok_or(AppError::NoPermission)?;
    if message.sender_id != session.user_id && !space_member.is_admin {
        return Err(AppError::NoPermission);
    }
    Message::delete(db, &id).await?;
    Event::message_deleted(message.channel_id, message.id);
    api::Return::new(&message).build()
}

async fn send_preview(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let NewPreview { id, channel_id, name, media_id, in_game, is_action, text, entities, whisper_to_users, start } = api::parse_body(req).await?;


    let mut conn = database::get().await;
    let db = &mut *conn;
    let channel_id = channel_id.clone();

    let member = ChannelMember::get(db, &session.user_id, &channel_id)
        .await?
        .ok_or(AppError::NoPermission)?;

    let preview = Preview {
        id,
        sender_id: session.user_id,
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
    };
    Event::message_preview(preview);
    api::Return::new(true).build()
}

async fn by_channel(req: Request<Body>) -> api::AppResult {
    let ByChannel { channel_id, before, amount } = parse_query(req.uri())?;

    let mut db = database::get().await;
    let db = &mut *db;

    Channel::get_by_id(db, &channel_id)
        .await?
        .ok_or(AppError::NotFound("channels"))?;
    let messages = Message::get_by_channel(db, &channel_id, before, amount).await?;
    api::Return::new(&messages).build()
}

pub async fn router(req: Request<Body>, path: &str) -> api::AppResult {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/query", Method::GET) => query(req).await,
        ("/by_channel", Method::GET) => by_channel(req).await,
        ("/send", Method::POST) => send(req).await,
        ("/delete", Method::POST) => delete(req).await,
        ("/edit", Method::POST) => edit(req).await,
        ("/preview", Method::POST) => send_preview(req).await,
        _ => Err(AppError::missing()),
    }
}

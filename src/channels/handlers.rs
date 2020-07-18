use super::api::{Create, Edit};
use super::models::ChannelMember;
use super::Channel;
use crate::channels::api::{ChannelWithMember, ChannelWithRelated, EditMember, JoinChannel};
use crate::channels::models::Member;
use crate::interface::{self, missing, ok_response, parse_body, parse_query, IdQuery, Response};
use crate::csrf::authenticate;
use crate::database;
use crate::database::Querist;
use crate::error::AppError;
use crate::events::Event;
use crate::spaces::{Space, SpaceMember};
use hyper::{Body, Request};
use uuid::Uuid;

async fn admin_only<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Result<(), AppError> {
    let member = SpaceMember::get(db, user_id, space_id)
        .await?
        .ok_or(AppError::NoPermission)?;
    if !member.is_admin {
        return Err(AppError::NoPermission);
    }
    Ok(())
}

async fn query(req: Request<Body>) -> Result<Channel, AppError> {
    let query: IdQuery = parse_query(req.uri())?;

    let mut db = database::get().await?;
    Channel::get_by_id(&mut *db, &query.id)
        .await?
        .ok_or(AppError::NotFound("channels"))
}

async fn query_with_related(req: Request<Body>) -> Result<ChannelWithRelated, AppError> {
    let query: IdQuery = parse_query(req.uri())?;

    let mut conn = database::get().await?;
    let db = &mut *conn;
    let (channel, space) = Channel::get_with_space(db, &query.id)
        .await?
        .ok_or(AppError::NotFound("channels"))?;
    let members = Member::get_by_channel(db, channel.id).await?;
    let color_list = ChannelMember::get_color_list(db, &channel.id).await?;
    let with_related = ChannelWithRelated {
        channel,
        space,
        members,
        color_list,
    };
    Ok(with_related)
}

async fn create(req: Request<Body>) -> Result<ChannelWithMember, AppError> {
    let session = authenticate(&req).await?;
    let Create {
        space_id,
        name,
        character_name,
        default_dice_type,
    } = interface::parse_body(req).await?;

    let mut conn = database::get().await?;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;
    Space::get_by_id(db, &space_id)
        .await?
        .ok_or_else(|| AppError::BadRequest(format!("The space not found")))?;
    admin_only(db, &session.user_id, &space_id).await?;

    let channel = Channel::create(
        db,
        &space_id,
        &*name,
        true,
        default_dice_type.as_ref().map(String::as_str),
    )
    .await?;
    let channel_member = ChannelMember::add_user(db, &session.user_id, &channel.id, &*character_name, true).await?;
    trans.commit().await?;
    let joined = ChannelWithMember {
        channel,
        member: channel_member,
    };
    Ok(joined)
}

async fn edit(req: Request<Body>) -> Result<bool, AppError> {
    let session = authenticate(&req).await?;
    let Edit {
        channel_id,
        name,
        topic,
        default_dice_type,
    } = interface::parse_body(req).await?;

    let mut conn = database::get().await?;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;

    let space_member = SpaceMember::get_by_channel(db, &session.user_id, &channel_id)
        .await?
        .ok_or_else(|| AppError::NoPermission)?;
    if !space_member.is_admin {
        return Err(AppError::NoPermission);
    }
    let channel = Channel::edit(
        db,
        &channel_id,
        name.as_ref().map(String::as_str),
        topic.as_ref().map(String::as_str),
        default_dice_type.as_ref().map(String::as_str),
    )
    .await?;
    trans.commit().await?;
    Event::channel_edited(channel);
    Ok(true)
}

async fn edit_member(req: Request<Body>) -> Result<ChannelMember, AppError> {
    let session = authenticate(&req).await?;
    let EditMember {
        channel_id,
        character_name,
        text_color,
    } = interface::parse_body(req).await?;

    let mut conn = database::get().await?;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;

    ChannelMember::get(db, &session.user_id, &channel_id)
        .await?
        .ok_or_else(|| AppError::NoPermission)?;

    let character_name = character_name.as_ref().map(String::as_str);
    let text_color = text_color.as_ref().map(String::as_str);
    let channel_member = ChannelMember::edit(db, session.user_id, channel_id, character_name, text_color).await?;
    trans.commit().await?;
    Event::push_members(channel_id);
    channel_member.ok_or(unexpected!("database returns no result when the user editing channel member."))
}

async fn members(req: Request<Body>) -> Result<Vec<ChannelMember>, AppError> {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut db = database::get().await?;
    let db = &mut *db;

    ChannelMember::get_by_channel(db, &id).await.map_err(Into::into)
}

async fn join(req: Request<Body>) -> Result<ChannelWithMember, AppError> {
    let session = authenticate(&req).await?;
    let JoinChannel {
        channel_id,
        character_name,
    } = parse_body(req).await?;
    let mut conn = database::get().await?;
    let db = &mut *conn;

    let channel = Channel::get_by_id(db, &channel_id)
        .await?
        .ok_or(AppError::NotFound("channels"))?;
    SpaceMember::get(db, &session.user_id, &channel.space_id)
        .await?
        .ok_or(AppError::NoPermission)?;
    let member = ChannelMember::add_user(db, &session.user_id, &channel.id, &*character_name, false).await?;
    Event::push_members(channel_id);
    Ok(ChannelWithMember { channel, member })
}

async fn leave(req: Request<Body>) -> Result<bool, AppError> {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;
    let mut db = database::get().await?;
    ChannelMember::remove_user(&mut *db, &session.user_id, &id).await?;
    Event::push_members(id);
    Ok(true)
}

async fn delete(req: Request<Body>) -> Result<bool, AppError> {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut conn = database::get().await?;
    let db = &mut *conn;

    let channel = Channel::get_by_id(db, &id)
        .await?
        .ok_or(AppError::NotFound("channels"))?;

    admin_only(db, &session.user_id, &channel.space_id).await?;

    Channel::delete(db, &id).await?;
    log::info!("channel {} was deleted.", &id);
    Event::channel_deleted(id);
    Ok(true)
}

async fn by_space(req: Request<Body>) -> Result<Vec<Channel>, AppError> {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut conn = database::get().await?;
    let db = &mut *conn;
    Channel::get_by_space(db, &id).await.map_err(Into::into)
}

async fn my_channels(req: Request<Body>) -> Result<Vec<ChannelWithMember>, AppError> {
    let session = authenticate(&req).await?;

    let mut conn = database::get().await?;
    let db = &mut *conn;
    Channel::get_by_user(db, session.user_id).await.map_err(Into::into)
}

pub async fn router(req: Request<Body>, path: &str) -> Result<Response, AppError> {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/query", Method::GET) => query(req).await.map(ok_response),
        ("/query_with_related", Method::GET) => query_with_related(req).await.map(ok_response),
        ("/by_space", Method::GET) => by_space(req).await.map(ok_response),
        ("/my", Method::GET) => my_channels(req).await.map(ok_response),
        ("/create", Method::POST) => create(req).await.map(ok_response),
        ("/edit", Method::POST) => edit(req).await.map(ok_response),
        ("/edit_member", Method::POST) => edit_member(req).await.map(ok_response),
        ("/members", Method::GET) => members(req).await.map(ok_response),
        ("/join", Method::POST) => join(req).await.map(ok_response),
        ("/leave", Method::POST) => leave(req).await.map(ok_response),
        ("/delete", Method::POST) => delete(req).await.map(ok_response),
        _ => missing(),
    }
}

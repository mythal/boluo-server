use super::api::{ChannelWithRelated, Create};
use super::models::ChannelMember;
use super::Channel;
use crate::api::{self, parse_query, IdQuery};
use crate::csrf::authenticate;
use crate::database;
use crate::error::AppError;
use crate::messages::Message;
use crate::spaces::{Space, SpaceMember};
use hyper::{Body, Request};

async fn query(req: Request<Body>) -> api::Result {
    let query: IdQuery = parse_query(req.uri())?;

    let mut db = database::get().await;
    let channel = Channel::get_by_id(&mut *db, &query.id).await?;
    return api::Return::new(&channel).build();
}

async fn create(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let form: Create = api::parse_body(req).await?;
    let mut conn = database::get().await;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;
    let space = Space::get_by_id(db, &form.space_id).await?;
    let member = SpaceMember::get(db, &session.user_id, &form.space_id).await;
    if let Some(member) = member {
        if member.is_admin {
            let channel = Channel::create(db, &form.space_id, &*form.name, true).await?;
            let channel_member = ChannelMember::add_user(db, &session.user_id, &channel.id).await?;
            trans.commit().await?;
            let channel_with_related = ChannelWithRelated {
                channel,
                members: vec![channel_member],
                space,
            };
            return api::Return::new(&channel_with_related).build();
        }
    }
    log::warn!(
        "The user {} failed to try create a channel in the space {}",
        session.user_id,
        space.id
    );
    Err(AppError::Unauthenticated)
}

async fn edit(req: Request<Body>) -> api::Result {
    todo!()
}

async fn members(req: Request<Body>) -> api::Result {
    let query: IdQuery = parse_query(req.uri())?;
    let mut db = database::get().await;
    let db = &mut *db;

    let members = ChannelMember::get_by_channel(db, &query.id).await?;
    api::Return::new(&members).build()
}

async fn messages(req: Request<Body>) -> api::Result {
    let query: IdQuery = parse_query(req.uri())?;

    let mut db = database::get().await;
    let db = &mut *db;

    let channel = Channel::get_by_id(db, &query.id).await?;
    let messages = Message::get_by_channel(db, &channel.id).await?;
    api::Return::new(&messages).build()
}

async fn join(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut conn = database::get().await;
    let db = &mut *conn;

    let channel = Channel::get_by_id(db, &id).await?;
    SpaceMember::get(db, &session.user_id, &channel.space_id)
        .await
        .ok_or(AppError::Unauthenticated)?;
    let member = ChannelMember::add_user(db, &session.user_id, &channel.id).await?;

    api::Return::new(&member).build()
}

async fn leave(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;
    let mut db = database::get().await;
    ChannelMember::remove_user(&mut *db, &session.user_id, &id).await?;
    api::Return::new(&true).build()
}

async fn delete(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut conn = database::get().await;
    let db = &mut *conn;

    let channel = Channel::get_by_id(db, &id).await?;
    let member = SpaceMember::get(db, &session.user_id, &channel.space_id).await;
    if let Some(member) = member {
        if member.is_admin {
            let deleted_channel = Channel::delete(db, &id).await?;
            log::info!("channel {} was deleted.", &id);
            return api::Return::new(&deleted_channel).build();
        }
    }
    log::warn!(
        "The user {} failed to try delete a channel {}",
        session.user_id,
        channel.id
    );
    Err(AppError::Unauthenticated)
}

pub async fn router(req: Request<Body>, path: &str) -> api::Result {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/query", Method::GET) => query(req).await,
        ("/create", Method::POST) => create(req).await,
        ("/edit", Method::POST) => edit(req).await,
        ("/members", Method::GET) => members(req).await,
        ("/messages", Method::GET) => messages(req).await,
        ("/join", Method::POST) => join(req).await,
        ("/leave", Method::POST) => leave(req).await,
        ("/delete", Method::DELETE) => delete(req).await,
        _ => Err(AppError::missing()),
    }
}

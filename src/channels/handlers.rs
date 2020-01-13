use super::api::{get_receiver, ChannelWithRelated, Create, Event};
use super::models::ChannelMember;
use super::Channel;
use crate::api::{self, parse_query, IdQuery};
use crate::channels::api::EventQueue;
use crate::csrf::authenticate;
use crate::database;
use crate::database::Querist;
use crate::error::AppError;
use crate::spaces::{Space, SpaceMember};
use crate::utils::timestamp;
use hyper::{Body, Request};
use uuid::Uuid;

async fn admin_only<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Result<(), AppError> {
    let member = SpaceMember::get(db, user_id, space_id)
        .await?
        .ok_or(AppError::Unauthenticated)?;
    if member.is_admin {
        return Err(AppError::Unauthenticated);
    }
    Ok(())
}

async fn query(req: Request<Body>) -> api::AppResult {
    let query: IdQuery = parse_query(req.uri())?;

    let mut db = database::get().await;
    let channel = Channel::get_by_id(&mut *db, &query.id).await?;
    return api::Return::new(&channel).build();
}

async fn create(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let Create { space_id, name } = api::parse_body(req).await?;
    let mut conn = database::get().await;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;
    let space = Space::get_by_id(db, &space_id)
        .await?
        .ok_or_else(|| AppError::BadRequest(format!("The space not found")))?;
    admin_only(db, &session.user_id, &space_id).await?;

    let channel = Channel::create(db, &space_id, &*name, true)
        .await?
        .ok_or(AppError::AlreadyExists)?;
    let channel_member = ChannelMember::add_user(db, &session.user_id, &channel.id).await?;
    trans.commit().await?;
    let channel_with_related = ChannelWithRelated {
        channel,
        members: vec![channel_member],
        space,
    };
    api::Return::new(&channel_with_related).build()
}

async fn edit(_req: Request<Body>) -> api::AppResult {
    todo!()
}

async fn members(req: Request<Body>) -> api::AppResult {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut db = database::get().await;
    let db = &mut *db;

    let members = ChannelMember::get_by_channel(db, &id).await?;
    api::Return::new(&members).build()
}

async fn join(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut conn = database::get().await;
    let db = &mut *conn;

    let channel = Channel::get_by_id(db, &id).await?.ok_or(AppError::NotFound)?;
    SpaceMember::get(db, &session.user_id, &channel.space_id)
        .await?
        .ok_or(AppError::Unauthenticated)?;
    let member = ChannelMember::add_user(db, &session.user_id, &channel.id).await?;

    api::Return::new(&member).build()
}

async fn leave(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;
    let mut db = database::get().await;
    ChannelMember::remove_user(&mut *db, &session.user_id, &id).await?;
    api::Return::new(&true).build()
}

async fn delete(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut conn = database::get().await;
    let db = &mut *conn;

    let channel = Channel::get_by_id(db, &id).await?.ok_or(AppError::NotFound)?;

    admin_only(db, &session.user_id, &channel.space_id).await?;

    Channel::delete(db, &id).await?;
    log::info!("channel {} was deleted.", &id);
    Event::channel_deleted(id).fire(channel.id);
    return api::Return::new(true).build();
}

async fn subscript(req: Request<Body>) -> api::AppResult {
    use tokio::sync::broadcast::RecvError;

    let IdQuery { id } = parse_query(req.uri())?;
    let mut rx = get_receiver(&id).await;
    let start = timestamp();
    loop {
        match rx.recv().await {
            Ok(()) | Err(RecvError::Lagged(_)) => {
                let queue = EventQueue::get().read().await;
                let events = queue.get_events(0, &id);
                if events.len() > 0 {
                    return api::Return::new(events).build();
                }
                let now = timestamp();
                if start + 6000 < now {
                    return api::Return::<Vec<()>>::new(vec![]).build();
                }
            }
            Err(RecvError::Closed) => return Err(unexpected!("The subscription channel was closed")),
        }
    }
}

async fn by_space(req: Request<Body>) -> api::AppResult {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let channels = Channel::get_by_space(db, &id).await?;
    return api::Return::new(&channels).build();
}

pub async fn router(req: Request<Body>, path: &str) -> api::AppResult {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/query", Method::GET) => query(req).await,
        ("/by_space", Method::GET) => by_space(req).await,
        ("/create", Method::POST) => create(req).await,
        ("/edit", Method::POST) => edit(req).await,
        ("/members", Method::GET) => members(req).await,
        ("/join", Method::POST) => join(req).await,
        ("/leave", Method::POST) => leave(req).await,
        ("/delete", Method::DELETE) => delete(req).await,
        ("/subscript", Method::GET) => subscript(req).await,
        _ => Err(AppError::missing()),
    }
}

use super::api::{fire, get_receiver, ChannelWithRelated, Create, Event};
use super::models::ChannelMember;
use super::Channel;
use crate::api::{self, parse_query, IdQuery};
use crate::channels::api::EventQueue;
use crate::csrf::authenticate;
use crate::database;
use crate::error::AppError;
use crate::messages::{user_id_and_whether_master, Message};
use crate::spaces::{Space, SpaceMember};
use crate::utils::timestamp;
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

async fn edit(_req: Request<Body>) -> api::Result {
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
    let IdQuery { id } = parse_query(req.uri())?;

    let mut db = database::get().await;
    let db = &mut *db;

    let (user_id, is_master) = user_id_and_whether_master(db, &req, &id).await;

    let channel = Channel::get_by_id(db, &id).await?;
    let mut messages = Message::get_by_channel(db, &channel.id).await?;
    if !is_master {
        for message in messages.iter_mut() {
            message.mask(user_id.as_ref());
        }
    }
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
            Channel::delete(db, &id).await?;
            log::info!("channel {} was deleted.", &id);
            fire(&channel.id, Event::channel_deleted(id));
            return api::Return::new(true).build();
        }
    }
    log::warn!(
        "The user {} failed to try delete a channel {}",
        session.user_id,
        channel.id
    );
    Err(AppError::Unauthenticated)
}

async fn subscript(req: Request<Body>) -> api::Result {
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
        ("/subscript", Method::GET) => subscript(req).await,
        _ => Err(AppError::missing()),
    }
}

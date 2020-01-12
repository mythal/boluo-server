use super::api::{Create, SpaceWithRelated};
use super::{Space, SpaceMember};
use crate::api::{self, parse_query, IdQuery};
use crate::channels::Channel;
use crate::csrf::authenticate;
use crate::database;
use crate::error::AppError;
use hyper::{Body, Request};

async fn list(_req: Request<Body>) -> api::AppResult {
    let mut conn = database::get().await;
    let spaces = Space::all(&mut *conn).await?;
    api::Return::new(&spaces).build()
}

async fn query(req: Request<Body>) -> api::AppResult {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let space = Space::get_by_id(db, &id).await?;
    return api::Return::new(&space).build();
}

async fn query_with_related(req: Request<Body>) -> api::AppResult {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let space = Space::get_by_id(db, &id).await?;
    let members = SpaceMember::get_by_space(db, &id).await?;
    let channels = Channel::get_by_space(db, &id).await?;
    let with_related = SpaceWithRelated {
        space,
        members,
        channels,
    };
    return api::Return::new(&with_related).build();
}

async fn create(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let form: Create = api::parse_body(req).await?;
    let mut conn = database::get().await;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;
    let password: Option<&str> = form.password.as_ref().map(|s| s.as_str());
    let space = Space::create(db, &*form.name, &session.user_id, password).await?;
    let member = SpaceMember::add_owner(db, &session.user_id, &space.id).await?;
    trans.commit().await?;
    let members = vec![member];
    let channels = vec![];
    log::info!("a channel ({}) was just created", space.id);
    api::Return::new(&SpaceWithRelated {
        space,
        members,
        channels,
    })
    .build()
}

async fn edit(_req: Request<Body>) -> api::AppResult {
    todo!()
}

async fn join(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut db = database::get().await;
    let db = &mut *db;

    Space::get_by_id(db, &id).await?;
    let user_id = &session.user_id;
    let member = SpaceMember::add_user(db, user_id, &id).await?;
    api::Return::new(&member).build()
}

async fn leave(req: Request<Body>) -> api::AppResult {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut conn = database::get().await;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;

    SpaceMember::remove_user(db, &session.user_id, &id).await?;
    trans.commit().await?;
    api::Return::new(&true).build()
}

async fn members(req: Request<Body>) -> api::AppResult {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut db = database::get().await;
    let db = &mut *db;
    let members = SpaceMember::get_by_space(&mut *db, &id).await?;
    api::Return::new(&members).build()
}

async fn delete(req: Request<Body>) -> api::AppResult {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let session = authenticate(&req).await?;
    let db = &mut *conn;
    let space = Space::get_by_id(db, &id).await?;
    if space.owner_id == session.user_id {
        Space::delete(db, &id).await?;
        log::info!("A space ({}) was deleted", space.id);
        return api::Return::new(&space).build();
    }
    log::warn!("The user {} failed to try delete a space {}", session.user_id, space.id);
    Err(AppError::Unauthenticated)
}

pub async fn router(req: Request<Body>, path: &str) -> api::AppResult {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/list", Method::GET) => query(req).await,
        ("/query", Method::GET) => query(req).await,
        ("/query_with_related", Method::GET) => query(req).await,
        ("/create", Method::POST) => create(req).await,
        ("/edit", Method::POST) => edit(req).await,
        ("/join", Method::POST) => join(req).await,
        ("/leave", Method::POST) => leave(req).await,
        ("/members", Method::POST) => members(req).await,
        ("/delete", Method::DELETE) => delete(req).await,
        _ => Err(AppError::missing()),
    }
}

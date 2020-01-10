use super::api::{Create, SpaceWithRelated};
use super::{Space, SpaceMember};
use crate::api::{self, parse_query, IdQuery};
use crate::csrf::authenticate;
use crate::database::{self, Querist};
use hyper::{Body, Request};
use uuid::Uuid;

async fn is_member<T: Querist>(db: &mut T, space: &Uuid, req: &Request<Body>) -> Result<SpaceMember, api::Error> {
    let session = authenticate(&req).await?;
    SpaceMember::fetch(db, &session.user_id, space).await.ok_or_else(api::Error::unauthorized)
}

async fn list(req: Request<Body>) -> api::Result {
    let mut conn = database::get().await;
    let spaces = Space::all(&mut *conn).await?;
    api::Return::new(&spaces).build()
}

async fn query(req: Request<Body>) -> api::Result {
    let query: IdQuery = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let space = Space::get_by_id(db, &query.id).await?;
    if !space.is_public {
        is_member(db, &space.id, &req).await?;
    }
    return api::Return::new(&space).build();
}

async fn query_with_related(req: Request<Body>) -> api::Result {
    let query: IdQuery = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let space = Space::get_by_id(db, &query.id).await?;
    if !space.is_public {
        is_member(db, &space.id, &req).await?;
    }
    let members = Space::members(db, &space.id).await?;
    let channels = Space::channels(db, &space.id).await?;
    let with_related = SpaceWithRelated {
        space,
        members,
        channels,
    };
    return api::Return::new(&with_related).build();
}

async fn create(req: Request<Body>) -> api::Result {
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
    log::info!("a channel was just created");
    api::Return::new(&SpaceWithRelated {
        space,
        members,
        channels,
    })
    .build()
}

async fn edit(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    todo!()
}

async fn join(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let query: IdQuery = parse_query(req.uri())?;

    let mut db = database::get().await;
    let db = &mut *db;

    let space = Space::get_by_id(db, &query.id).await?;
    if !space.is_public {
        return Err(api::Error::unauthorized());
    }
    let user_id = &session.user_id;
    let space_id = &space.id;
    let member = SpaceMember::add_user(db, user_id, space_id).await?;
    api::Return::new(&member).build()
}

async fn leave(req: Request<Body>) -> api::Result {
    let session = authenticate(&req).await?;
    let query: IdQuery = parse_query(req.uri())?;

    let mut db = database::get().await;
    let db = &mut *db;

    SpaceMember::remove_user(db, &session.user_id, &query.id).await?;
    api::Return::new(&true).build()
}

async fn members(req: Request<Body>) -> api::Result {
    let query: IdQuery = parse_query(req.uri())?;
    let mut db = database::get().await;
    let db = &mut *db;
    if !Space::is_public(db, &query.id).await? {
        is_member(db, &query.id, &req).await?;
    }
    let members = Space::members(&mut *db, &query.id).await?;
    api::Return::new(&members).build()
}

async fn channels(req: Request<Body>) -> api::Result {
    let query: IdQuery = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    if !Space::is_public(db, &query.id).await? {
        is_member(db, &query.id, &req).await?;
    }
    let channels = Space::channels(db, &query.id).await?;
    return api::Return::new(&channels).build();
}

async fn delete(req: Request<Body>) -> api::Result {
    let query: IdQuery = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let session = authenticate(&req).await?;
    let db = &mut *conn;
    let space = Space::get_by_id(db, &query.id).await?;
    if space.owner_id == session.user_id {
        Space::delete(db, &query.id).await?;
        log::info!("a space ({}) was deleted", space.id);
        return api::Return::new(&space).build();
    }
    Err(api::Error::unauthorized())
}

pub async fn router(req: Request<Body>, path: &str) -> api::Result {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/list/", Method::GET) => query(req).await,
        ("/query/", Method::GET) => query(req).await,
        ("/query_with_related/", Method::GET) => query(req).await,
        ("/create/", Method::POST) => create(req).await,
        ("/edit/", Method::POST) => edit(req).await,
        ("/join/", Method::POST) => join(req).await,
        ("/leave/", Method::POST) => leave(req).await,
        ("/members/", Method::POST) => members(req).await,
        ("/channels/", Method::POST) => channels(req).await,
        ("/delete/", Method::DELETE) => delete(req).await,
        _ => Err(api::Error::not_found()),
    }
}

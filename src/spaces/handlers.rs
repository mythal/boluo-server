use super::api::{Create, Edit, SpaceWithRelated};
use super::{Space, SpaceMember};
use crate::common::{self, parse_query, IdQuery, Response, missing, ok_response};
use crate::channels::Channel;
use crate::csrf::authenticate;
use crate::database;
use crate::error::AppError;
use crate::spaces::api::SpaceWithMember;
use hyper::{Body, Request};

async fn list(_req: Request<Body>) -> Result<Vec<Space>, AppError> {
    let mut conn = database::get().await;
    Space::all(&mut *conn).await.map_err(Into::into)
}

async fn query(req: Request<Body>) -> Result<Space, AppError> {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    Space::get_by_id(db, &id).await?.ok_or(AppError::NotFound("space"))
}

async fn query_with_related(req: Request<Body>) -> Result<SpaceWithRelated, AppError> {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    let space = Space::get_by_id(db, &id).await?.ok_or(AppError::NotFound("spaces"))?;
    let members = SpaceMember::get_by_space(db, &id).await?;
    let channels = Channel::get_by_space(db, &id).await?;
    Ok(SpaceWithRelated {
        space,
        members,
        channels,
    })
}

async fn my_spaces(req: Request<Body>) -> Result<Vec<SpaceWithMember>, AppError> {
    let session = authenticate(&req).await?;
    let mut conn = database::get().await;
    let db = &mut *conn;
    Space::get_by_user(db, session.user_id).await.map_err(Into::into)
}

async fn create(req: Request<Body>) -> Result<SpaceWithMember, AppError> {
    let session = authenticate(&req).await?;
    let Create { name, password, description, default_dice_type }: Create = common::parse_body(req).await?;

    let mut conn = database::get().await;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;
    let space = Space::create(db, name, &session.user_id, description, password, default_dice_type).await?;
    let member = SpaceMember::add_admin(db, &session.user_id, &space.id).await?;
    trans.commit().await?;
    log::info!("a channel ({}) was just created", space.id);
    Ok(SpaceWithMember { space, member })
}

async fn edit(req: Request<Body>) -> Result<Space, AppError> {
    let session = authenticate(&req).await?;
    let Edit { space_id, name, description, default_dice_type }: Edit = common::parse_body(req).await?;

    let mut conn = database::get().await;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;

    let space_member = SpaceMember::get(db, &session.user_id, &space_id)
        .await?
        .ok_or(AppError::NoPermission)?;
    if !space_member.is_admin {
        return Err(AppError::NoPermission);
    }
    let space = Space::edit(db, space_id, name, description, default_dice_type)
        .await?
        .ok_or_else(|| unexpected!("No such space found."))?;
    trans.commit().await?;
    Ok(space)
}

async fn join(req: Request<Body>) -> Result<SpaceWithMember, AppError> {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut db = database::get().await;
    let db = &mut *db;

    let space = Space::get_by_id(db, &id).await?.ok_or(AppError::NotFound("spaces"))?;
    let user_id = &session.user_id;
    let member = if &space.owner_id == user_id {
        SpaceMember::add_admin(db, user_id, &id).await?
    } else {
        SpaceMember::add_user(db, user_id, &id).await?
    };
    Ok(SpaceWithMember { space, member })
}

async fn leave(req: Request<Body>) -> Result<bool, AppError> {
    let session = authenticate(&req).await?;
    let IdQuery { id } = parse_query(req.uri())?;

    let mut conn = database::get().await;
    let mut trans = conn.transaction().await?;
    let db = &mut trans;

    SpaceMember::remove_user(db, &session.user_id, &id).await?;
    trans.commit().await?;
    Ok(true)
}

async fn members(req: Request<Body>) -> Result<Vec<SpaceMember>, AppError> {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut db = database::get().await;
    let db = &mut *db;
    SpaceMember::get_by_space(&mut *db, &id).await.map_err(Into::into)
}

async fn delete(req: Request<Body>) -> Result<Space, AppError> {
    let IdQuery { id } = parse_query(req.uri())?;
    let mut conn = database::get().await;
    let session = authenticate(&req).await?;
    let db = &mut *conn;
    let space = Space::get_by_id(db, &id).await?.ok_or(AppError::NotFound("spaces"))?;
    if space.owner_id == session.user_id {
        Space::delete(db, &id).await?;
        log::info!("A space ({}) was deleted", space.id);
        return Ok(space)
    }
    log::warn!("The user {} failed to try delete a space {}", session.user_id, space.id);
    Err(AppError::NoPermission)
}

pub async fn router(req: Request<Body>, path: &str) -> Result<Response, AppError> {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/list", Method::GET) => list(req).await.map(ok_response),
        ("/query", Method::GET) => query(req).await.map(ok_response),
        ("/query_with_related", Method::GET) => query_with_related(req).await.map(ok_response),
        ("/my", Method::GET) => my_spaces(req).await.map(ok_response),
        ("/create", Method::POST) => create(req).await.map(ok_response),
        ("/edit", Method::POST) => edit(req).await.map(ok_response),
        ("/join", Method::POST) => join(req).await.map(ok_response),
        ("/leave", Method::POST) => leave(req).await.map(ok_response),
        ("/members", Method::GET) => members(req).await.map(ok_response),
        ("/delete", Method::POST) => delete(req).await.map(ok_response),
        _ => missing(),
    }
}

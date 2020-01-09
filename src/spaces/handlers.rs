use super::Space;
use crate::api::{self, Error, IdQuery};
use crate::database;
use hyper::{Body, Request};

async fn query(req: Request<Body>) -> api::Result {
    let query = IdQuery::from_request(&req)?;
    if let IdQuery { id: Some(id), .. } = query {
        let mut db = database::get().await;
        let space = Space::get_by_id(&mut *db, &id).await?;
        return api::Return::new(&space).build();
    }
    Err(Error::bad_request())
}

async fn create(req: Request<Body>) -> api::Result {
    todo!()
}

async fn edit(req: Request<Body>) -> api::Result {
    todo!()
}

async fn join(req: Request<Body>) -> api::Result {
    todo!()
}

async fn leave(req: Request<Body>) -> api::Result {
    todo!()
}

async fn members(req: Request<Body>) -> api::Result {
    todo!()
}

async fn channels(req: Request<Body>) -> api::Result {
    todo!()
}

async fn delete(req: Request<Body>) -> api::Result {
    todo!()
}

pub async fn router(req: Request<Body>, path: &str) -> api::Result {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/", Method::GET) => query(req).await,
        ("/", Method::POST) => create(req).await,
        ("/edit/", Method::POST) => edit(req).await,
        ("/join/", Method::POST) => join(req).await,
        ("/leave/", Method::POST) => leave(req).await,
        ("/members/", Method::POST) => members(req).await,
        ("/channels/", Method::POST) => channels(req).await,
        ("/delete/", Method::DELETE) => delete(req).await,
        _ => Err(api::Error::not_found()),
    }
}

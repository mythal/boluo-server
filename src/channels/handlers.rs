use super::Channel;
use crate::api::{self, Error, IdQuery};
use crate::database;
use hyper::{Body, Request};

async fn query(req: Request<Body>) -> api::Result {
    let query = IdQuery::from_request(&req)?;
    if let IdQuery { id: Some(id), .. } = query {
        let mut db = database::get().await;
        let channel = Channel::get_by_id(&mut *db, &id).await?;
        return api::Return::new(&channel).build();
    }
    Err(Error::bad_request())
}

async fn create(req: Request<Body>) -> api::Result {
    todo!()
}

async fn edit(req: Request<Body>) -> api::Result {
    todo!()
}

async fn members(req: Request<Body>) -> api::Result {
    todo!()
}

async fn messages(req: Request<Body>) -> api::Result {
    todo!()
}

async fn join(req: Request<Body>) -> api::Result {
    todo!()
}

async fn leave(req: Request<Body>) -> api::Result {
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
        ("/members/", Method::GET) => members(req).await,
        ("/messages/", Method::GET) => messages(req).await,
        ("/join/", Method::POST) => join(req).await,
        ("/leave/", Method::POST) => leave(req).await,
        ("/delete/", Method::DELETE) => delete(req).await,
        _ => Err(api::Error::not_found()),
    }
}

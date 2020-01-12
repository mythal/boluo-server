use crate::api;
use hyper::{Body, Request};

async fn upload(req: Request<Body>) -> api::Result {
    todo!()
}

async fn query(req: Request<Body>) -> api::Result {
    todo!()
}

async fn delete(req: Request<Body>) -> api::Result {
    todo!()
}

pub async fn router(req: Request<Body>, path: &str) -> api::Result {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/", Method::GET) => query(req).await,
        ("/", Method::POST) => upload(req).await,
        ("/", Method::DELETE) => delete(req).await,
        _ => Err(api::Error::not_found()),
    }
}

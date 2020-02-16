use super::api::EventQuery;
use super::Event;
use crate::api::{parse_query, AppResult, Return};
use crate::error::AppError;
use hyper::{Body, Request};

async fn events(req: Request<Body>) -> AppResult {
    let EventQuery { mailbox, after } = parse_query(req.uri())?;
    let events = Event::get_from_cache(&mailbox, after).await?;
    Return::new(events).build()
}

async fn subscribe(req: Request<Body>) -> AppResult {
    let EventQuery { mailbox, after } = parse_query(req.uri())?;
    Event::wait(mailbox).await;
    let events = Event::get_from_cache(&mailbox, after).await?;
    Return::new(events).build()
}

pub async fn router(req: Request<Body>, path: &str) -> AppResult {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/subscribe", Method::GET) => subscribe(req).await,
        ("/events", Method::GET) => events(req).await,
        _ => Err(AppError::missing()),
    }
}

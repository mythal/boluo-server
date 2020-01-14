use super::api::EventQuery;
use super::{Event, EventQueue, LOCAL_EVENTS_CACHE_TIME_MS};
use crate::api::{parse_query, AppResult, Return};
use crate::error::AppError;
use crate::utils::timestamp;
use hyper::{Body, Request};

async fn events(req: Request<Body>) -> AppResult {
    let EventQuery { mailbox, since } = parse_query(req.uri())?;
    let events = Event::get_from_cache(&mailbox, since).await?;
    Return::new(events).build()
}

async fn subscribe(req: Request<Body>) -> AppResult {
    let EventQuery { mailbox, since } = parse_query(req.uri())?;
    let delta = timestamp() - since;
    if delta > LOCAL_EVENTS_CACHE_TIME_MS / 2 {
        let events = Event::get_from_cache(&mailbox, since).await?;
        return Return::new(events).build();
    }

    Event::wait(mailbox).await;

    let events = EventQueue::get_events(since, &mailbox).await;
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

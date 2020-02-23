use super::api::EventQuery;
use super::Event;
use crate::common::{parse_query, Response, missing, ok_response};
use crate::error::AppError;
use std::time::Duration;
use hyper::{Body, Request};
use tokio::time::delay_for;
use tokio::select;


async fn events(req: Request<Body>) -> Result<Vec<Event>, AppError> {
    let EventQuery { mailbox, after } = parse_query(req.uri())?;
    Event::get_from_cache(&mailbox, after).await.map_err(Into::into)
}

async fn subscribe(req: Request<Body>) -> Result<Vec<Event>, AppError> {
    let EventQuery { mailbox, after } = parse_query(req.uri())?;
    let wait_events = Event::wait(mailbox);
    let timeout = delay_for(Duration::from_secs(8));
    let events = select! {
        _ = wait_events => Event::get_from_cache(&mailbox, after).await?,
        _ = timeout => vec![],
    };
    Ok(events)
}

pub async fn router(req: Request<Body>, path: &str) -> Result<Response, AppError> {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/subscribe", Method::GET) => subscribe(req).await.map(ok_response),
        ("/events", Method::GET) => events(req).await.map(ok_response),
        _ => missing(),
    }
}

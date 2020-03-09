#![allow(dead_code)]

use std::env;
use std::net::SocketAddr;

use crate::context::debug;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server, Uri};

#[macro_use]
mod utils;
#[macro_use]
mod error;
mod date_format;
mod common;
mod cache;
mod channels;
mod context;
mod cors;
mod csrf;
mod database;
mod events;
mod logger;
mod media;
mod messages;
mod pool;
mod session;
mod spaces;
mod users;
mod websocket;
mod validators;

use crate::common::{Response, missing, ok_response, err_response};
use crate::error::AppError;
use crate::cors::allow_origin;

async fn router(req: Request<Body>) -> Result<Response, AppError> {
    let path = req.uri().path().to_string();
    macro_rules! table {
        ($prefix: expr, $handler: expr) => {
            let prefix = $prefix;
            if path.starts_with(prefix) {
                return $handler(req, &path[prefix.len()..]).await;
            }
        };
    }
    if path == "/api/csrf-token" {
        return csrf::get_csrf_token(req).await.map(ok_response);
    }
    table!("/api/messages", messages::router);
    table!("/api/users", users::router);
    table!("/api/media", media::router);
    table!("/api/channels", channels::router);
    table!("/api/spaces", spaces::router);
    table!("/api/events", events::router);
    missing()
}

fn log_error(e: &AppError, method: &str, uri: &Uri, elapsed: u128) {
    use std::error::Error;
    use AppError::*;
    match e {
        NotFound(_) | Conflict(_) =>
            log::debug!("{:>6} {} {}ms - {}", method, uri, elapsed, e),
        Validation(_) | BadRequest(_) | MethodNotAllowed =>
            log::info!("{:>6} {} {}ms - {}", method, uri, elapsed, e),
        e => {
            if let Some(source) = e.source() {
                log::error!("{:>6} {} {}ms - {} - source: {:?}", method, uri, elapsed, e, source)
            } else {
                log::error!("{:>6} {} {}ms - {}", method, uri, elapsed, e)
            }
        }
    }
}

async fn handler(req: Request<Body>) -> Result<Response, hyper::Error> {
    use std::time::SystemTime;
    let start = SystemTime::now();
    let method = req.method().clone();
    let method = method.as_str();
    let uri = req.uri().clone();
    if context::debug() && req.method() == hyper::Method::OPTIONS {
        return Ok(cors::preflight_requests(req));
    }
    let mut response = router(req).await;
    if debug() {
        response = response.map(allow_origin);
    }
    let elapsed = SystemTime::now().duration_since(start).unwrap().as_millis();
    match response {
        Ok(response) => {
            log::debug!("{:>6} {} {}ms", method, uri, elapsed);
            Ok(response)
        },
        Err(e) => {
            log_error(&e, method, &uri, elapsed);
            Ok(err_response(e))
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let port: u16 = env::var("PORT").unwrap().parse().unwrap();
    logger::setup_logger(debug()).unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let make_svc = make_service_fn(|_: &AddrStream| async { Ok::<_, hyper::Error>(service_fn(handler)) });

    let server = Server::bind(&addr).serve(make_svc);
    tokio::spawn(events::tasks::periodical_cleaner());
    // Run this server for... forever!
    if let Err(e) = server.await {
        log::error!("server error: {}", e);
    }
}

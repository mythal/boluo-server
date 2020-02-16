#![allow(dead_code)]

use std::env;
use std::net::SocketAddr;

use crate::context::debug;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

#[macro_use]
mod utils;
#[macro_use]
mod error;
mod date_format;
mod api;
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
mod validators;

use crate::events::periodical_cleaner;
use error::AppError;

async fn router(req: Request<Body>) -> api::AppResult {
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
        return csrf::get_csrf_token(req).await;
    }
    table!("/api/messages", messages::router);
    table!("/api/users", users::router);
    table!("/api/media", media::router);
    table!("/api/channels", channels::router);
    table!("/api/spaces", spaces::router);
    table!("/api/events", events::router);
    Err(AppError::missing())
}

fn error_response(e: AppError) -> Response<Body> {
    use std::error::Error;
    if debug() {
        if let Some(source) = e.source() {
            log::debug!("{} Source: {:?}", &e, source);
        } else {
            log::debug!("{}", &e);
        }
    }

    fn last_resort(e2: AppError) -> Response<Body> {
        log::error!("An error occurred while processing the error: {}", e2);
        Response::builder()
            .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("An error occurred while processing the error."))
            .unwrap()
    }

    api::Return::<String>::form_error(e).build().unwrap_or_else(last_resort)
}

async fn handler(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    use std::time::SystemTime;
    let start = SystemTime::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    if context::debug() && req.method() == hyper::Method::OPTIONS {
        return Ok(cors::preflight_requests(req));
    }
    let mut response = router(req).await.unwrap_or_else(error_response);
    if debug() {
        response = cors::allow_origin(response);
    }
    let elapsed = SystemTime::now().duration_since(start).unwrap();
    log::info!("{:>4}ms {:>5} {}", elapsed.as_millis(), method.as_str(), uri);
    Ok(response)
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let port: u16 = env::var("PORT").unwrap().parse().unwrap();
    logger::setup_logger(debug()).unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let make_svc = make_service_fn(|_: &AddrStream| async { Ok::<_, hyper::Error>(service_fn(handler)) });

    let server = Server::bind(&addr).serve(make_svc);
    tokio::spawn(periodical_cleaner());
    // Run this server for... forever!
    if let Err(e) = server.await {
        log::error!("server error: {}", e);
    }
}

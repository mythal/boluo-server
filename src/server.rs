#![feature(backtrace)]
#![allow(dead_code)]

use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::context::debug;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server};

#[macro_use]
mod utils;
#[macro_use]
mod error;
mod cache;
mod channels;
mod context;
mod cors;
mod csrf;
mod database;
mod date_format;
mod events;
mod interface;
mod logger;
mod media;
mod mail;
mod messages;
mod pool;
mod pos;
mod session;
mod spaces;
mod users;
mod validators;
mod websocket;

use crate::cors::allow_origin;
use crate::error::AppError;
use crate::interface::{err_response, missing, ok_response, Response};
use once_cell::sync::OnceCell;

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

async fn handler(req: Request<Body>) -> Result<Response, hyper::Error> {
    use std::time::Instant;
    let start = Instant::now();
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
    let mut has_error = false;
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            has_error = true;
            error::log_error(&e, &uri);
            err_response(e)
        }
    };

    if has_error {
        log::warn!("{} {} {:?}", method, &uri, start.elapsed());
    } else if uri.path().starts_with("/api/users/get_me") {
        log::debug!("{} {} {:?}", method, &uri, start.elapsed());
    } else {
        log::info!("{} {} {:?}", method, &uri, start.elapsed());
    }
    Ok(response)
}

static SENTRY: OnceCell<sentry::ClientInitGuard> = OnceCell::new();

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let port: u16 = env::var("PORT").unwrap().parse().unwrap();
    logger::setup_logger(debug()).unwrap();
    if let Ok(dsn) = env::var("SENTRY_DSN") {
        SENTRY
            .set(sentry::init((
                dsn,
                sentry::ClientOptions {
                    release: sentry::release_name!(),
                    debug: debug(),
                    environment: Some(if debug() { "development" } else { "production" }.into()),
                    ..Default::default()
                },
            )))
            .ok();
    };

    let addr: Ipv4Addr = env::var("HOST").unwrap_or("127.0.0.1".to_string()).parse().unwrap();
    let addr = SocketAddr::new(IpAddr::V4(addr), port);

    let make_svc = make_service_fn(|_: &AddrStream| async { Ok::<_, hyper::Error>(service_fn(handler)) });

    let server = Server::bind(&addr).serve(make_svc);
    events::tasks::start();
    // Run this server for... forever!
    if let Err(e) = server.await {
        log::error!("server error: {}", e);
    }
}

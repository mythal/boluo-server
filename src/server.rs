#![allow(dead_code)]

use std::env;
use std::net::SocketAddr;

use crate::context::debug;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

#[macro_use]
mod utils;
mod api;
mod channels;
mod context;
mod database;
mod handlers;
mod media;
mod messages;
mod session;
mod spaces;
mod users;
mod validators;

fn cors_allow_origin(mut res: Response<Body>) -> Response<Body> {
    use hyper::header::{HeaderValue, ACCESS_CONTROL_ALLOW_ORIGIN};
    let header = res.headers_mut();
    header.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    res
}

fn preflight_requests(res: Request<Body>) -> Response<Body> {
    use hyper::header::{
        HeaderValue, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_REQUEST_HEADERS,
    };

    let headers = res.headers();
    let allow_headers = headers
        .get(ACCESS_CONTROL_REQUEST_HEADERS)
        .map(Clone::clone)
        .unwrap_or(HeaderValue::from_static(""));
    let response = Response::builder()
        .header(
            ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH"),
        )
        .header(ACCESS_CONTROL_ALLOW_HEADERS, allow_headers)
        .body(Body::empty())
        .unwrap();
    cors_allow_origin(response)
}

async fn router(req: Request<Body>) -> api::Result {
    let path = req.uri().path().to_string();

    let users_prefix = "/api/users";
    if path.starts_with(users_prefix) {
        return handlers::users(req, &path[users_prefix.len()..]).await;
    }
    Err(api::Error::not_found())
}

async fn handler(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    println!("{} {}", req.method(), req.uri());
    if context::debug() && req.method() == hyper::Method::OPTIONS {
        return Ok(preflight_requests(req));
    }
    let mut response = router(req).await.unwrap_or_else(|e| e.build());
    if debug() {
        response = cors_allow_origin(response);
    }
    Ok(response)
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    context::init().await;
    let port: u16 = env::var("PORT").unwrap().parse().unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let make_svc = make_service_fn::<_, AddrStream, _>(move |_| async { Ok::<_, hyper::Error>(service_fn(handler)) });

    let server = Server::bind(&addr).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

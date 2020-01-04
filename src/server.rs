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
mod cors;
mod database;
mod handlers;
mod media;
mod messages;
mod session;
mod spaces;
mod users;
mod validators;

async fn router(req: Request<Body>) -> api::Result {
    let path = req.uri().path().to_string();

    let users_prefix = "/api/users";
    if path.starts_with(users_prefix) {
        return handlers::users(req, &path[users_prefix.len()..]).await;
    }
    Err(api::Error::not_found())
}

async fn handler(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    use std::time::SystemTime;
    print!("{} {} ", req.method(), req.uri());
    let start = SystemTime::now();
    if context::debug() && req.method() == hyper::Method::OPTIONS {
        return Ok(cors::preflight_requests(req));
    }
    let mut response = router(req).await.unwrap_or_else(|e| e.build());
    if debug() {
        response = cors::allow_origin(response);
    }
    let elapsed = SystemTime::now().duration_since(start).unwrap();
    println!("{}ms", elapsed.as_millis());
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

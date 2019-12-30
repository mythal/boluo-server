#![allow(dead_code)]

use std::env;
use std::net::SocketAddr;

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

#[macro_use]
mod utils;
mod api;
mod channels;
mod context;
mod database;
mod media;
mod messages;
mod spaces;
mod users;
mod validators;
mod handlers;

async fn router(req: Request<Body>) -> api::Result {
    let path = req.uri().path();

    if path.starts_with("/api/users") {
        return handlers::users(req).await;
    }
    Err(api::Error::not_found())
}

async fn handler(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    Ok(router(req).await.unwrap_or_else(|e| e.build()))
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

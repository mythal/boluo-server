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
mod csrf;
mod database;
mod media;
mod messages;
mod session;
mod spaces;
mod users;
mod validators;

async fn router(req: Request<Body>) -> api::Result {
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
    table!("/api/users", users::router);
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

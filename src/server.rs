#![allow(dead_code)]

use std::env;
use std::net::SocketAddr;

use futures::TryStreamExt as _;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use once_cell::sync::OnceCell;

mod channels;
mod database;
mod media;
mod messages;
mod spaces;
mod users;

#[derive(Clone)]
pub struct Context {
    pub pool: database::pool::Pool,
}

pub static CTX: OnceCell<Context> = OnceCell::new();

impl Context {
    async fn new() -> Context {
        Context {
            pool: database::pool::Pool::with_num(10).await,
        }
    }

    fn get() -> &'static Context {
        CTX.get().unwrap()
    }
}

async fn router(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let _context = Context::get();
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::from("Try POSTing data to /echo");
        }
        (&Method::POST, "/echo") => {
            *response.body_mut() = req.into_body();
        }
        (&Method::POST, "/echo/uppercase") => {
            // This is actually a new `futures::Stream`...
            let mapping = req
                .into_body()
                .map_ok(|chunk| chunk.iter().map(|byte| byte.to_ascii_uppercase()).collect::<Vec<u8>>());

            // Use `Body::wrap_stream` to convert it to a `Body`...
            *response.body_mut() = Body::wrap_stream(mapping);
        }
        (&Method::POST, "/echo/reverse") => {
            // Await the full body to be concatenated into a single `Bytes`...
            let full_body = hyper::body::to_bytes(req.into_body()).await?;

            // Iterate the full body in reverse order and collect into a new Vec.
            let reversed = full_body.iter().rev().cloned().collect::<Vec<u8>>();

            *response.body_mut() = reversed.into();
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(response)
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let port: u16 = env::var("PORT").unwrap().parse().unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    CTX.set(Context::new().await).ok();

    let make_svc = make_service_fn::<_, AddrStream, _>(move |_| async { Ok::<_, hyper::Error>(service_fn(router)) });

    let server = Server::bind(&addr).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

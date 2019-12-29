#![allow(dead_code)]

use std::env;
use std::net::SocketAddr;

use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};

mod channels;
mod database;
mod media;
mod messages;
mod spaces;
mod users;
mod api;
mod context;


async fn register(req: Request<Body>) -> api::Result {
    use database::CreationError;
    if hyper::Method::POST != req.method() {
        return Err(api::Error::method_not_allowed());
    }
    let body = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(|_| api::Error::bad_request())?;
    let form: users::RegisterForm = serde_json::from_slice(&*body)
        .map_err(|_| api::Error::bad_request())?;
    let user = context::pool()
        .run(|mut db| async move {
            (form.register(&mut db).await, db)
        })
        .await
        .map_err(|e| {
            match e {
                CreationError::AlreadyExists => api::Error::new("This e-mail or username already exists.", StatusCode::CONFLICT),
                e => e.into()
            }
        })?;
    api::Return::new(&user)
        .status(StatusCode::CREATED)
        .build()
}

async fn router(req: Request<Body>) -> api::Result {
    let path = req.uri().path();


    if path == "/api" {
        let response = Response::new(Body::from("Hello, world"));
        return Ok(response);
    }
    if path == "/api/users/register" {
        return register(req).await;
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

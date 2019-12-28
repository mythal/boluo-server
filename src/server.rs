#![allow(dead_code)]

use std::env;
use std::net::SocketAddr;

use hyper::{Body, Request, Response, Server};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::StatusCode;
use once_cell::sync::OnceCell;
use serde::Serialize;

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

pub type HttpRequest = hyper::Request<hyper::Body>;
pub type HttpResult<T> = Result<hyper::Response<T>, hyper::Error>;

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

async fn not_found(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::from("Not found requested resources."));
    *response.status_mut() = StatusCode::NOT_FOUND;
    Ok(response)
}


fn bad_request() -> Response<Body> {
    let mut response = Response::new(Body::from("Bad request."));
    *response.status_mut() = StatusCode::BAD_REQUEST;
    response
}

fn internal_error() -> Response<Body> {
    let mut response = Response::new(Body::from("Server internal error."));
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    response
}

fn error(msg: &'static str, status: StatusCode) -> Response<Body> {
    let body = Body::from(msg);
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
}

fn json_response<T: Serialize>(value: &T) -> Response<Body> {
    match serde_json::to_vec(value) {
        Ok(bytes) => {
            Response::new(Body::from(bytes))
        },
        Err(_) => internal_error(),
    }
}

async fn register(req: Request<Body>) -> Response<Body> {
    use database::CreationError;

    let body = hyper::body::to_bytes(req.into_body()).await.ok().and_then(|body_bytes| {
        Some(serde_json::from_slice::<users::RegisterForm>(&*body_bytes).ok()?)
    });
    if body.is_none() {
        return bad_request();
    }
    let form = body.unwrap();

    let context = Context::get();
    let register_result = context.pool.run(|mut db| async move {
        let register_result = form.register(&mut db).await;
        (register_result, db)
    }).await;
    match register_result {
        Ok(user) => json_response(&user),
        Err(CreationError::AlreadyExists) => error("This e-mail or username already exists.", StatusCode::CONFLICT),
        _ => internal_error()
    }
}

async fn router(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();


    if path == "/api" {
        let response = Response::new(Body::from("Hello, world"));
        return Ok(response);
    }
    if path == "/api/users/register" {
        return Ok(register(req).await);
    }

    not_found(req).await
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

//! Types and functions for to help building APIs.
use std::convert::From;
use std::error::Error as StdError;
use std::fmt;
use std::result::Result as StdResult;
use std::time;

use hyper::{Body, Response, StatusCode};
use serde::export::fmt::Display;
use serde::{Deserialize, Serialize};

use crate::context::debug;
use crate::database::{CreationError, DbError, FetchError};
use crate::session::Unauthenticated;

pub type Request = hyper::Request<hyper::Body>;
pub type Result = std::result::Result<hyper::Response<hyper::Body>, Error>;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    #[serde(rename = "type")]
    kind: &'static str,
    pub message: String,
    pub status_code: u16,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl StdError for Error {}

impl Error {
    pub fn new<T: ToString>(message: T, status: StatusCode) -> Error {
        Error {
            kind: "error",
            message: message.to_string(),
            status_code: status.as_u16(),
        }
    }

    pub fn not_found() -> Error {
        Error::new("Not found requested resources.", StatusCode::NOT_FOUND)
    }

    pub fn internal() -> Error {
        Error::new("Server internal error.", StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn bad_request() -> Error {
        Error::new("Bad request.", StatusCode::BAD_REQUEST)
    }

    pub fn method_not_allowed() -> Error {
        Error::new("Method not allowed", StatusCode::METHOD_NOT_ALLOWED)
    }

    pub fn unauthorized() -> Error {
        Error::new("Unauthorized", StatusCode::UNAUTHORIZED)
    }
    pub fn unexpected(e: &dyn StdError) -> Error {
        let mut error = Error::internal();
        if debug() {
            error.message = e.to_string();
        }
        error
    }

    pub fn build(&self) -> Response<Body> {
        let bytes = serde_json::to_vec(self).unwrap_or_else(|_| serde_json::to_vec(&Error::internal()).unwrap());
        let status = StatusCode::from_u16(self.status_code).expect("invalid struct code");
        Response::builder()
            .status(status)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(Body::from(bytes))
            .expect("failed to build response")
    }
}

impl From<CreationError> for Error {
    fn from(e: CreationError) -> Error {
        match e {
            CreationError::EmptyResult => Error::new("This record already exists.", StatusCode::CONFLICT),
            CreationError::ValidationFail(message) => Error::new(message, StatusCode::FORBIDDEN),
            e => Error::unexpected(&e),
        }
    }
}

impl From<FetchError> for Error {
    fn from(e: FetchError) -> Error {
        match e {
            FetchError::NoSuchRecord => Error::new("Record not found.", StatusCode::NOT_FOUND),
            FetchError::NoPermission => Error::new("You have no permission to access.", StatusCode::UNAUTHORIZED),
            e => Error::unexpected(&e),
        }
    }
}

impl From<DbError> for Error {
    fn from(e: DbError) -> Error {
        log::warn!("a unexpected database error: {}", e);
        Error::internal()
    }
}

#[derive(Serialize, Debug)]
pub struct Return<'a, T: Serialize> {
    value: &'a T,
    #[serde(rename = "type")]
    kind: &'static str,
    status_code: u16,
    delta: Option<f64>,
}

impl<'a, T: Serialize> Return<'a, T> {
    pub fn new(value: &'a T) -> Return<'a, T> {
        Return {
            value,
            kind: "return",
            status_code: 200,
            delta: None,
        }
    }

    pub fn status(self, s: StatusCode) -> Return<'a, T> {
        let status_code = s.as_u16();
        Return { status_code, ..self }
    }

    pub fn start_at(self, t: time::SystemTime) -> Return<'a, T> {
        let now = time::SystemTime::now();
        let delta = Some(now.duration_since(t).unwrap().as_secs_f64());
        Return { delta, ..self }
    }

    pub fn build(&self) -> Result {
        let bytes = serde_json::to_vec(self).map_err(|_| Error::bad_request())?;

        Response::builder()
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .status(StatusCode::from_u16(self.status_code).unwrap())
            .body(Body::from(bytes))
            .map_err(|_| Error::internal())
    }
}

impl From<Unauthenticated> for Error {
    fn from(_: Unauthenticated) -> Error {
        Error::unauthorized()
    }
}

pub fn parse_query<T>(uri: &hyper::http::Uri) -> StdResult<T, Error>
where
    for<'de> T: Deserialize<'de>,
{
    let query = uri.query().unwrap_or("");
    match serde_urlencoded::from_str(query) {
        Ok(r) => Ok(r),
        Err(e) => {
            log::debug!("failed to parse uri ({}): {}", uri, e);
            Err(Error::bad_request())
        }
    }
}

pub(crate) async fn parse_body<T>(req: hyper::Request<Body>) -> StdResult<T, Error>
where
    for<'de> T: Deserialize<'de>,
{
    let body = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(|_| Error::bad_request())?;
    serde_json::from_slice(&*body).map_err(|_| Error::bad_request())
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct IdQuery {
    pub id: uuid::Uuid,
}

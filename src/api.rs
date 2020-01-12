//! Types and functions for to help building APIs.
use std::result::Result as StdResult;

use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub type Request = hyper::Request<hyper::Body>;
pub type Result = std::result::Result<hyper::Response<hyper::Body>, AppError>;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Return<T: Serialize> {
    value: T,
    #[serde(rename = "type")]
    kind: &'static str,
    status_code: u16,
}

impl<T: Serialize> Return<T> {
    pub fn new(value: T) -> Return<T> {
        Return {
            value,
            kind: "return",
            status_code: 200,
        }
    }

    pub fn form_error(e: &AppError) -> Return<String> {
        Return {
            value: e.to_string(),
            kind: "error",
            status_code: e.status_code().as_u16(),
        }
    }

    pub fn status(self, s: StatusCode) -> Return<T> {
        let status_code = s.as_u16();
        Return { status_code, ..self }
    }

    pub fn build(&self) -> Result {
        let bytes = serde_json::to_vec(self).map_err(unexpected!())?;

        Response::builder()
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .status(StatusCode::from_u16(self.status_code).unwrap())
            .body(Body::from(bytes))
            .map_err(unexpected!())
    }
}

pub fn parse_query<T>(uri: &hyper::http::Uri) -> StdResult<T, AppError>
where
    for<'de> T: Deserialize<'de>,
{
    let query = uri.query().unwrap_or("");
    serde_urlencoded::from_str(query).map_err(|e| {
        let message = format!("Failed to parse the query in the URI ({})", uri);
        log::debug!("{}: {}", message, e);
        AppError::BadRequest(message)
    })
}

pub async fn parse_body<T>(req: hyper::Request<Body>) -> StdResult<T, AppError>
where
    for<'de> T: Deserialize<'de>,
{
    let body = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(|_| AppError::BadRequest(format!("Failed to read the request body")))?;
    serde_json::from_slice(&*body).map_err(|_| AppError::BadRequest(format!("Failed to parse the request body")))
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct IdQuery {
    pub id: uuid::Uuid,
}

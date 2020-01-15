//! Types and functions for to help building APIs.
use std::result::Result as StdResult;

use hyper::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub type Request = hyper::Request<hyper::Body>;
pub type AppResult = std::result::Result<hyper::Response<hyper::Body>, AppError>;

#[derive(Debug)]
pub struct Return<T: Serialize> {
    result: Result<T, AppError>,
    status_code: StatusCode,
}

impl<T: Serialize> Return<T> {
    pub fn new(value: T) -> Return<T> {
        Return {
            result: Ok(value),
            status_code: StatusCode::OK,
        }
    }

    pub fn form_error(e: AppError) -> Return<String> {
        Return {
            status_code: e.status_code(),
            result: Err(e),
        }
    }

    pub fn status(self, status_code: StatusCode) -> Return<T> {
        Return { status_code, ..self }
    }

    pub fn build(self) -> AppResult {
        let return_body = match self.result {
            Ok(some) => WebResult {
                ok: true,
                some: Some(some),
                err: None,
            },
            Err(err) => WebResult {
                ok: true,
                some: None,
                err: Some(err.to_string()),
            },
        };

        let bytes = serde_json::to_vec(&return_body).map_err(unexpected!())?;

        Response::builder()
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .status(self.status_code)
            .body(Body::from(bytes))
            .map_err(unexpected!())
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WebResult<T: Serialize> {
    ok: bool,
    some: Option<T>,
    err: Option<String>,
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

#[test]
fn test_get_uuid() {
    use hyper::Uri;
    use uuid::Uuid;

    let uuid = Uuid::new_v4();
    let path_and_query = format!("/?id={}", uuid.to_string());
    let uri = Uri::builder().path_and_query(&*path_and_query).build().unwrap();
    let query: IdQuery = parse_query(&uri).unwrap();
    assert_eq!(query.id, uuid);

    let uri = Uri::builder().path_and_query("/?id=&").build().unwrap();
    let query = parse_query::<IdQuery>(&uri);
    assert!(query.is_err());
}

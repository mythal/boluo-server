use super::api::{Login, Register};
use super::models::User;
use crate::csrf::authenticate as csrf_auth;
use crate::database;
use crate::session::Unauthenticated::Unexpected;
use crate::{api, context};
use hyper::http::uri::Uri;
use hyper::{Body, Method, Request, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct IdQuery {
    id: Option<Uuid>,
}

fn get_query<T>(uri: &Uri) -> Option<T>
where
    for<'de> T: Deserialize<'de>,
{
    let query = uri.query()?;
    serde_urlencoded::from_str(query).ok()
}

#[test]
fn test_get_uuid() {
    let uuid = Uuid::new_v4();
    let path_and_query = format!("/?id={}", uuid.to_string());
    let uri = Uri::builder().path_and_query(&*path_and_query).build().unwrap();
    let query: IdQuery = get_query(&uri).unwrap();
    assert_eq!(query.id, Some(uuid));

    let uri = Uri::builder().path_and_query("/?id=&").build().unwrap();
    let query = get_query::<IdQuery>(&uri);
    assert_eq!(query, None);
}

async fn parse_body<T>(req: Request<Body>) -> Result<T, api::Error>
where
    for<'de> T: Deserialize<'de>,
{
    let body = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(|_| api::Error::bad_request())?;
    serde_json::from_slice(&*body).map_err(|_| api::Error::bad_request())
}

async fn register(req: Request<Body>) -> api::Result {
    let form: Register = parse_body(req).await?;
    let mut db = database::get().await;
    let user = form.register(&mut *db).await?;
    log::info!("{} ({}) was registered.", user.username, user.email);
    api::Return::new(&user).status(StatusCode::CREATED).build()
}

pub async fn get_users(query: IdQuery) -> api::Result {
    if let IdQuery { id: Some(id), .. } = query {
        let mut db = database::get().await;
        let user = User::get_by_id(&mut *db, &id).await?;
        return api::Return::new(&user).build();
    }
    Err(api::Error::not_found())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginReturn {
    user: User,
    token: Option<String>,
}

pub async fn login(req: Request<Body>) -> api::Result {
    use crate::session;
    use cookie::{CookieBuilder, SameSite};
    use database::FetchError::NoPermission;
    use hyper::header::{HeaderValue, SET_COOKIE};

    let form: Login = parse_body(req).await?;
    let mut db = database::get().await;
    let login = form.login(&mut *db).await;
    if let Err(NoPermission) = login {
        log::warn!("Someone failed to try to login: {}", form.username);
    }
    let user = login?;
    let expires = time::now() + time::Duration::days(256);
    let session = session::start(&user.id).await.ok_or(Unexpected)?;
    let token = session::token(&session);
    let session_cookie = CookieBuilder::new("session", token.clone())
        .same_site(SameSite::Lax)
        .secure(!context::debug())
        .http_only(true)
        .path("/api/")
        .expires(expires)
        .finish()
        .to_string();

    let token = if form.with_token { Some(token) } else { None };
    let login_return = LoginReturn { user, token };

    let mut response = api::Return::new(&login_return).build()?;
    let headers = response.headers_mut();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&*session_cookie).unwrap());
    Ok(response)
}

pub async fn router(req: Request<Body>, path: &str) -> api::Result {
    if path == "/login" && req.method() == Method::POST {
        return login(req).await;
    }
    if path == "/register" && req.method() == Method::POST {
        return register(req).await;
    }
    if req.method() == Method::GET {
        csrf_auth(&req).await?;
        let query = get_query::<IdQuery>(req.uri()).ok_or_else(api::Error::bad_request)?;
        return get_users(query).await;
    }
    Err(api::Error::method_not_allowed())
}

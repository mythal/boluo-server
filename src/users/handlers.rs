use super::api::{Login, Register, LoginReturn};
use super::models::User;
use crate::database;
use crate::session::Unauthenticated::Unexpected;
use crate::{api, context};
use hyper::{Body, Method, Request, StatusCode};
use serde::Deserialize;
use uuid::Uuid;
use crate::session::revoke_session;
use once_cell::sync::OnceCell;

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct IdQuery {
    id: Option<Uuid>,
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

pub async fn query_user(query: IdQuery) -> api::Result {
    if let IdQuery { id: Some(id), .. } = query {
        let mut db = database::get().await;
        let user = User::get_by_id(&mut *db, &id).await?;
        return api::Return::new(&user).build();
    }
    Err(api::Error::not_found())
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

pub async fn logout(req: Request<Body>) -> api::Result {
    use cookie::CookieBuilder;
    use hyper::header::{SET_COOKIE, HeaderValue};
    use crate::session::authenticate;

    if let Ok(session) = authenticate(&req).await {
        revoke_session(&session.id).await;
    }
    let mut response = api::Return::new(&true).build()?;
    let header = response.headers_mut();

    static HEADER_VALUE: OnceCell<HeaderValue> = OnceCell::new();
    let header_value = HEADER_VALUE.get_or_init(|| {
        let cookie = CookieBuilder::new("session", "")
            .http_only(true)
            .path("/api/")
            .expires(time::empty_tm())
            .finish()
            .to_string();
        HeaderValue::from_str(&*cookie).unwrap()
    });
    header.append(SET_COOKIE, header_value.clone());
    Ok(response)
}

pub async fn router(req: Request<Body>, path: &str) -> api::Result {
    match (path, req.method().clone()) {
        ("/login", Method::POST) => login(req).await,
        ("/register", Method::POST) => register(req).await,
        ("/logout", _) => logout(req).await,
        ("/", Method::GET) => {
            let query = api::parse_query::<IdQuery>(req.uri())
                .ok_or_else(api::Error::bad_request)?;
            query_user(query).await
        },
        _ => Err(api::Error::not_found())
    }
}

#[test]
fn test_get_uuid() {
    use hyper::Uri;
    let uuid = Uuid::new_v4();
    let path_and_query = format!("/?id={}", uuid.to_string());
    let uri = Uri::builder().path_and_query(&*path_and_query).build().unwrap();
    let query: IdQuery = api::parse_query(&uri).unwrap();
    assert_eq!(query.id, Some(uuid));

    let uri = Uri::builder().path_and_query("/?id=&").build().unwrap();
    let query = api::parse_query::<IdQuery>(&uri);
    assert_eq!(query, None);
}

use crate::context::pool;
use crate::users::{RegisterForm, User};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
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
    let form: RegisterForm = parse_body(req).await?;
    let user = context::pool()
        .run(|mut db| async move { (form.register(&mut db).await, db) })
        .await?;
    api::Return::new(&user).status(StatusCode::CREATED).build()
}

pub async fn get_users(query: IdQuery) -> api::Result {
    let pool = context::pool();
    if let IdQuery { id: Some(id), .. } = query {
        let user = pool
            .run(|mut db| async move { (User::get_by_id(&mut db, &id).await, db) })
            .await?;
        return api::Return::new(&user).build();
    }
    Err(api::Error::not_found())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsrfToken {
    csrf_token: String,
}

pub async fn login(req: Request<Body>) -> api::Result {
    use crate::session::SessionMap;
    use cookie::{CookieBuilder, SameSite};
    use hyper::header::{HeaderValue, SET_COOKIE};

    let form: LoginForm = parse_body(req).await?;
    let username = form.username.clone();
    let user = pool()
        .run(|mut db| async move { (User::get_by_username(&mut db, &*username).await, db) })
        .await?;
    let expires = time::now() + time::Duration::days(256);
    let session = SessionMap::get().start(&user.id).await;

    let session_cookie = CookieBuilder::new("session", session.key.to_string())
        .same_site(SameSite::Lax)
        .secure(!context::debug())
        .http_only(true)
        .path("/")
        .expires(expires)
        .finish()
        .to_string();

    let mut response = api::Return::new(&CsrfToken {
        csrf_token: session.csrf_token.to_string(),
    })
    .build()?;
    let headers = response.headers_mut();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&*session_cookie).unwrap());
    Ok(response)
}

pub async fn users(req: Request<Body>, path: &str) -> api::Result {
    if path == "/login" && req.method() == Method::POST {
        return login(req).await;
    }
    if path == "/register" && req.method() == Method::POST {
        return register(req).await;
    }
    if req.method() == Method::GET {
        let query = get_query::<IdQuery>(req.uri()).ok_or(api::Error::bad_request())?;
        return get_users(query).await;
    }
    Err(api::Error::method_not_allowed())
}

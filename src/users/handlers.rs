use super::api::{Login, LoginReturn, Register};
use super::models::User;
use crate::api::{parse_query, IdQuery, parse_body};
use crate::database;
use crate::session::revoke_session;

use crate::error::AppError;
use crate::{api, context};
use hyper::{Body, Method, Request, StatusCode};
use once_cell::sync::OnceCell;
use crate::users::api::Edit;
use crate::error::AppError::ValidationFail;

async fn register(req: Request<Body>) -> api::AppResult {
    let form: Register = api::parse_body(req).await?;
    let mut db = database::get().await;
    let user = form.register(&mut *db).await?;
    log::info!("{} ({}) was registered.", user.username, user.email);
    api::Return::new(user).status(StatusCode::CREATED).build()
}

pub async fn query_user(req: Request<Body>) -> api::AppResult {
    let query: IdQuery = parse_query(req.uri())?;

    let mut db = database::get().await;
    let user = User::get_by_id(&mut *db, &query.id).await?;
    api::Return::new(user).build()
}

pub async fn login(req: Request<Body>) -> api::AppResult {
    use crate::session;
    use cookie::{CookieBuilder, SameSite};
    use hyper::header::{HeaderValue, SET_COOKIE};

    let form: Login = api::parse_body(req).await?;
    let mut db = database::get().await;
    let login = form.login(&mut *db).await;
    if let Err(AppError::NoPermission) = &login {
        log::warn!("Someone failed to try to login: {}", form.username);
    }
    let user = login?;
    let expires = time::now() + time::Duration::days(256);
    let session = session::start(&user.id).await.map_err(unexpected!())?;
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

pub async fn logout(req: Request<Body>) -> api::AppResult {
    use crate::session::authenticate;
    use cookie::CookieBuilder;
    use hyper::header::{HeaderValue, SET_COOKIE};

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

pub async fn edit(req: Request<Body>) -> api::AppResult {
    use crate::csrf::authenticate;
    let session = authenticate(&req).await?;
    let edit: Edit = parse_body(req).await?;
    edit.validate().map_err(|msg| ValidationFail(msg.to_string()))?;
    let Edit { nickname, bio, avatar } = edit;

    let mut db = database::get().await;
    let user = User::set(&mut *db, &session.id, nickname, bio, avatar).await?;
    api::Return::new(user).build()
}

pub async fn router(req: Request<Body>, path: &str) -> api::AppResult {
    match (path, req.method().clone()) {
        ("/login", Method::POST) => login(req).await,
        ("/register", Method::POST) => register(req).await,
        ("/logout", _) => logout(req).await,
        ("/query", Method::GET) => query_user(req).await,
        ("/edit", Method::POST) => edit(req).await,
        _ => Err(AppError::missing()),
    }
}

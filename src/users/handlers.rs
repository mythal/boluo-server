use super::api::{Login, LoginReturn, Register, ResetPassword, ResetPasswordConfirm, ResetPasswordTokenCheck};
use super::models::User;
use crate::interface::{missing, ok_response, parse_body, parse_query, Response};
use crate::session::{remove_session, revoke_session};
use crate::{cache, database, mail};

use crate::channels::Channel;
use crate::context::debug;
use crate::error::{AppError, Find, ValidationFailed};
use crate::interface;
use crate::media::{upload, upload_params};
use crate::spaces::Space;
use crate::users::api::{CheckEmailExists, CheckUsernameExists, Edit, GetMe, QueryUser};
use crate::users::models::UserExt;
use crate::utils::get_ip;
use hyper::{Body, Method, Request};
use once_cell::sync::OnceCell;
use redis::AsyncCommands;

async fn register(req: Request<Body>) -> Result<User, AppError> {
    let Register {
        email,
        username,
        nickname,
        password,
    }: Register = interface::parse_body(req).await?;
    let mut db = database::get().await?;
    let user = User::register(&mut *db, &*email, &*username, &*nickname, &*password).await?;
    log::info!("{} ({}) was registered.", user.username, user.email);
    Ok(user)
}

pub async fn query_user(req: Request<Body>) -> Result<User, AppError> {
    use crate::session::authenticate;

    let QueryUser { id } = parse_query(req.uri())?;

    let id = if let Some(id) = id {
        id
    } else {
        authenticate(&req).await?.user_id
    };

    let mut db = database::get().await?;
    User::get_by_id(&mut *db, &id).await.or_not_found()
}

pub async fn get_me(req: Request<Body>) -> Result<Option<GetMe>, AppError> {
    use crate::session::authenticate;
    if let Ok(session) = authenticate(&req).await {
        let mut conn = database::get().await?;
        let db = &mut *conn;
        let user = User::get_by_id(db, &session.user_id).await?;
        if let Some(user) = user {
            let my_spaces = Space::get_by_user(db, &user.id).await?;
            let my_channels = Channel::get_by_user(db, user.id).await?;
            let settings = UserExt::get_settings(db, user.id).await?;
            Ok(Some(GetMe {
                user,
                settings,
                my_channels,
                my_spaces,
            }))
        } else {
            remove_session(session.id).await?;
            log::warn!("session is valid, but user can't be found at database.");
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

pub async fn login(req: Request<Body>) -> Result<Response, AppError> {
    use crate::session;
    use cookie::{CookieBuilder, SameSite};
    use hyper::header::{HeaderValue, SET_COOKIE};
    let is_developer = req.headers().contains_key("development");
    let form: Login = interface::parse_body(req).await?;
    let mut conn = database::get().await?;
    let db = &mut *conn;
    let user = User::login(db, &*form.username, &*form.password)
        .await
        .or_no_permission()?;
    let session = session::start(&user.id).await.map_err(error_unexpected!())?;
    let token = session::token(&session);
    let session_cookie = CookieBuilder::new("session", token.clone())
        .same_site(SameSite::Lax)
        .secure(!is_developer && !debug())
        .http_only(true)
        .path("/api/")
        .permanent()
        .finish()
        .to_string();

    let token = if form.with_token { Some(token) } else { None };
    let my_spaces = Space::get_by_user(db, &user.id).await?;
    let my_channels = Channel::get_by_user(db, user.id).await?;
    let settings = UserExt::get_settings(db, user.id).await?;
    let me = GetMe {
        user,
        settings,
        my_spaces,
        my_channels,
    };
    let mut response = ok_response(LoginReturn { me, token });
    let headers = response.headers_mut();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&*session_cookie).unwrap());
    Ok(response)
}

pub async fn logout(req: Request<Body>) -> Result<Response, AppError> {
    use crate::session::authenticate;
    use cookie::CookieBuilder;
    use hyper::header::{HeaderValue, SET_COOKIE};

    if let Ok(session) = authenticate(&req).await {
        revoke_session(&session.id).await?;
    }
    let mut response = ok_response(true);
    let header = response.headers_mut();

    static HEADER_VALUE: OnceCell<HeaderValue> = OnceCell::new();
    let header_value = HEADER_VALUE.get_or_init(|| {
        let cookie = CookieBuilder::new("session", "")
            .http_only(true)
            .path("/api/")
            .expires(time::OffsetDateTime::now_utc())
            .finish()
            .to_string();
        HeaderValue::from_str(&*cookie).unwrap()
    });
    header.append(SET_COOKIE, header_value.clone());
    Ok(response)
}

pub async fn edit(req: Request<Body>) -> Result<User, AppError> {
    use crate::csrf::authenticate;
    let session = authenticate(&req).await?;
    let Edit { nickname, bio, avatar }: Edit = parse_body(req).await?;
    let mut db = database::get().await?;
    User::edit(&mut *db, &session.user_id, nickname, bio, avatar)
        .await
        .map_err(Into::into)
}

pub fn is_image(mime: &Option<String>) -> bool {
    if let Some(mime) = mime {
        if mime == r"image/png" || mime == r"image/gif" || mime == r"image/jpeg" {
            return true;
        }
    }
    false
}

pub async fn update_settings(req: Request<Body>) -> Result<serde_json::Value, AppError> {
    use crate::csrf::authenticate;
    let session = authenticate(&req).await?;
    let settings: serde_json::Value = parse_body(req).await?;
    let mut db = database::get().await?;
    UserExt::update_settings(&mut *db, session.user_id, settings)
        .await
        .map_err(Into::into)
}

pub async fn edit_avatar(req: Request<Body>) -> Result<User, AppError> {
    use crate::csrf::authenticate;
    let session = authenticate(&req).await?;
    let params = upload_params(req.uri())?;
    if !is_image(&params.mime_type) {
        return Err(ValidationFailed("Incorrect File Format").into());
    }
    let media = upload(req, params, 1024 * 1024).await?;
    let mut db = database::get().await?;
    let media = media.create(&mut *db, session.user_id, "avatar").await?;
    User::edit(&mut *db, &session.user_id, None, None, Some(media.id))
        .await
        .map_err(Into::into)
}

pub async fn check_email_exists(req: Request<Body>) -> Result<bool, AppError> {
    let CheckEmailExists { email } = parse_query(req.uri())?;
    let mut db = database::get().await?;
    let user = User::get_by_email(&mut *db, &*email).await?;
    Ok(user.is_some())
}

pub async fn check_username_exists(req: Request<Body>) -> Result<bool, AppError> {
    let CheckUsernameExists { username } = parse_query(req.uri())?;
    let mut db = database::get().await?;
    let user = User::get_by_username(&mut *db, &*username).await?;
    Ok(user.is_some())
}

pub fn token_key(token: &str) -> Vec<u8> {
    let mut key = b"reset_password:".to_vec();
    key.extend_from_slice(token.as_bytes());
    return key;
}

pub async fn ip_limit(cache: &mut cache::Connection, req: &Request<Body>) -> Result<(), AppError> {
    let ip = get_ip(req).unwrap_or_else(|| {
        log::warn!("Unable to obtain client IP");
        log::info!("{:?}", req.headers());
        "0.0.0.0"
    });
    let mut key = b"reset_password_ip:".to_vec();
    key.extend_from_slice(ip.as_bytes());
    let counter: i32 = cache.inner.incr(&key, 1).await?;
    if counter == 1 {
        cache.inner.expire(&key, 60 * 2).await?;
    }
    if counter > 3 {
        return Err(AppError::LimitExceeded("IP"));
    }
    Ok(())
}

pub async fn email_limit(cache: &mut cache::Connection, email: &String) -> Result<(), AppError> {
    let email_key = token_key(email);
    let counter: i32 = cache.inner.incr(&email_key, 1).await?;
    if counter == 1 {
        cache.inner.expire(&email_key, 60 * 2).await?;
    }
    if counter > 2 {
        return Err(AppError::LimitExceeded("email"));
    }
    Ok(())
}

pub async fn reset_password(req: Request<Body>) -> Result<(), AppError> {
    let mut cache = cache::conn().await;
    ip_limit(&mut cache, &req).await?;
    let ResetPassword { email } = parse_body(req).await?;
    email_limit(&mut cache, &email).await?;

    let mut db = database::get().await?;
    User::get_by_email(&mut *db, &email)
        .await?
        .ok_or(AppError::NotFound("email"))?;
    let token = uuid::Uuid::new_v4().to_string();
    let key = token_key(&token);

    cache
        .set_with_expiration(key.as_slice(), email.as_bytes(), 60 * 60)
        .await?;
    mail::send(
        &email,
        "Boluo password reset",
        &format!(
            "
            <p>
                You have requested to reset your password.
                <a href=\"https://boluo.chat/confirm-password-reset/{}\">Click here</a> to reset your password.
            </p>
            <p>If you did not request to reset your password, please ignore this email.</p>
        ",
            token
        ),
    )
    .await
    .map_err(|e| AppError::Unexpected(e))?;
    Ok(())
}

pub async fn reset_password_token_check(req: Request<Body>) -> Result<bool, AppError> {
    let ResetPasswordTokenCheck { token } = parse_query(req.uri())?;
    let email = cache::conn()
        .await
        .get(token_key(&token).as_slice())
        .await?
        .map(String::from_utf8);
    if let Some(Ok(_)) = email {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn reset_password_confirm(req: Request<Body>) -> Result<(), AppError> {
    let ResetPasswordConfirm { token, password } = parse_body(req).await?;
    let mut db = database::get().await?;
    let email = cache::conn()
        .await
        .get(token_key(&token).as_slice())
        .await?
        .map(String::from_utf8)
        .ok_or_else(|| AppError::NotFound("token"))?
        .map_err(|e| AppError::Unexpected(e.into()))?;
    let user = User::get_by_email(&mut *db, &email)
        .await?
        .ok_or_else(|| AppError::NotFound("user"))?;
    User::reset_password(&mut *db, user.id, &password).await?;
    Ok(())
}

pub async fn router(req: Request<Body>, path: &str) -> Result<Response, AppError> {
    match (path, req.method().clone()) {
        ("/login", Method::POST) => login(req).await,
        ("/register", Method::POST) => register(req).await.map(ok_response),
        ("/logout", _) => logout(req).await,
        ("/query", Method::GET) => query_user(req).await.map(ok_response),
        ("/get_me", Method::GET) => get_me(req).await.map(ok_response),
        ("/edit", Method::POST) => edit(req).await.map(ok_response),
        ("/edit_avatar", Method::POST) => edit_avatar(req).await.map(ok_response),
        ("/update_settings", Method::POST) => update_settings(req).await.map(ok_response),
        ("/check_username", Method::GET) => check_username_exists(req).await.map(ok_response),
        ("/check_email", Method::GET) => check_email_exists(req).await.map(ok_response),
        ("/reset_password", Method::POST) => reset_password(req).await.map(ok_response),
        ("/reset_password_token_check", Method::GET) => reset_password_token_check(req).await.map(ok_response),
        ("/reset_password_confirm", Method::POST) => reset_password_confirm(req).await.map(ok_response),
        _ => missing(),
    }
}

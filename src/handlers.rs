use hyper::{Request, Body, StatusCode, Method};
use hyper::http::uri::Uri;
use crate::{api, context};
use crate::users::{RegisterForm, User};
use uuid::Uuid;
use serde::{Deserialize};


#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct IdQuery {
    id: Option<Uuid>,
}

fn get_query<T>(uri: &Uri) -> Option<T>
where for<'de> T: Deserialize<'de>
{
    let query = uri.query()?;
    serde_urlencoded::from_str(query).ok()
}

#[test]
fn test_get_uuid() {
    let uuid = Uuid::new_v4();
    let path_and_query = format!("/?id={}", uuid.to_string());
    let uri = Uri::builder()
        .path_and_query(&*path_and_query)
        .build()
        .unwrap();
    let query: IdQuery = get_query(&uri).unwrap();
    assert_eq!(query.id, Some(uuid));

    let uri = Uri::builder()
        .path_and_query("/?id=&")
        .build()
        .unwrap();
    let query = get_query::<IdQuery>(&uri);
    assert_eq!(query, None);
}

async fn register(req: Request<Body>) -> api::Result {
    let body = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(|_| api::Error::bad_request())?;
    let form: RegisterForm = serde_json::from_slice(&*body).map_err(|_| api::Error::bad_request())?;
    let user = context::pool()
        .run(|mut db| async move { (form.register(&mut db).await, db) })
        .await?;
    api::Return::new(&user).status(StatusCode::CREATED).build()
}

pub async fn get_users(query: IdQuery) -> api::Result {
    let pool = context::pool();
    if let IdQuery {id: Some(id), ..} = query {
        let user = pool.run(|mut db| async move {
            (User::get_by_id(&mut db, &id).await, db)
        }).await?;
        return api::Return::new(&user).build();
    }
    Err(api::Error::not_found())
}

pub async fn users(req: Request<Body>) -> api::Result {
    if req.method() == Method::POST {
        return register(req).await;
    }
    if req.method() == Method::GET {
        let query = get_query::<IdQuery>(req.uri())
            .ok_or(api::Error::bad_request())?;
        return get_users(query).await;
    }
    Err(api::Error::method_not_allowed())
}

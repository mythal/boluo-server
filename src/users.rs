use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::{CreationError, FetchError, Querist, query};

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[postgres(name = "users")]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub nickname: String,
    pub password: String,
    pub bio: String,
    pub joined: chrono::naive::NaiveDateTime,
    pub deactivated: bool,
    pub avatar_id: Option<Uuid>,
}

impl User {
    pub async fn all<T: Querist>(db: &mut T) -> Result<Vec<User>, tokio_postgres::Error> {
        let rows = db.query(query::SELECT_USERS.key, &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn create<T: Querist>(
        db: &mut T,
        email: &str,
        username: &str,
        nickname: &str,
        password: &str,
    ) -> Result<User, CreationError> {
        db.create(query::CREATE_USER.key, &[&email, &username, &nickname, &password])
            .await
            .map(|row| row.get(0))
    }

    async fn get<T: Querist>(
        db: &mut T,
        id: Option<&Uuid>,
        email: Option<&str>,
        username: Option<&str>,
    ) -> Result<User, FetchError> {
        db.fetch(query::FETCH_USER.key, &[&id, &email, &username])
            .await
            .map(|row| row.get(0))
    }

    pub async fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<User, FetchError> {
        User::get(db, Some(id), None, None).await
    }

    pub async fn get_by_email<T: Querist>(db: &mut T, email: &str) -> Result<User, FetchError> {
        User::get(db, None, Some(email), None).await
    }

    pub async fn get_by_username<T: Querist>(db: &mut T, username: &str) -> Result<User, FetchError> {
        User::get(db, None, None, Some(username)).await
    }

    pub async fn delete_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<User, FetchError> {
        db.fetch(query::DELETE_USER.key, &[id]).await.map(|row| row.get(0))
    }
}

#[tokio::test]
async fn user_test() {
    use crate::database::Client;

    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let email = "humura@humura.net";
    let username = "humura";
    let nickname = "Akami Humura";
    let password = "MadokaMadokaSuHaSuHa";
    let new_user = User::create(&mut trans, email, username, nickname, password)
        .await
        .unwrap();
    let user = User::get_by_id(&mut trans, &new_user.id).await.unwrap();
    assert_eq!(user.email, email);
    let user = User::get_by_email(&mut trans, &new_user.email).await.unwrap();
    assert_eq!(user.nickname, nickname);

    let deleted_user = User::delete_by_id(&mut trans, &new_user.id).await.unwrap();
    assert_eq!(deleted_user.id, user.id);

    let all_users = User::all(&mut trans).await.unwrap();
    assert!(all_users.into_iter().find(|u| u.id == user.id).is_none());
}

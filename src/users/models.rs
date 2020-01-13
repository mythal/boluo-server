use postgres_types::FromSql;
use serde::Serialize;
use uuid::Uuid;

use crate::database::Querist;
use crate::error::{AppError, DbError};
use crate::utils::inner_map;

#[derive(Debug, Serialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "users")]
pub struct User {
    pub id: Uuid,
    #[serde(skip)]
    pub email: String,
    pub username: String,
    pub nickname: String,
    #[serde(skip)]
    pub password: String,
    pub bio: String,
    pub joined: chrono::naive::NaiveDateTime,
    pub deactivated: bool,
    pub avatar_id: Option<Uuid>,
}

impl User {
    pub async fn all<T: Querist>(db: &mut T) -> Result<Vec<User>, DbError> {
        let rows = db.query_typed(include_str!("sql/all.sql"), &[], &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn create<T: Querist>(
        db: &mut T,
        email: &str,
        username: &str,
        nickname: &str,
        password: &str,
    ) -> Result<User, AppError> {
        use crate::validators::{EMAIL, NICKNAME, PASSWORD, USERNAME};

        let username = username.trim();
        let nickname = nickname.trim();
        let email = email.to_ascii_lowercase();

        let e = |s: &str| AppError::ValidationFail(s.to_string());

        EMAIL.run(&email).map_err(e)?;
        NICKNAME.run(&nickname).map_err(e)?;
        USERNAME.run(&username).map_err(e)?;
        PASSWORD.run(&password).map_err(e)?;

        let mut rows = db
            .query(
                include_str!("sql/create.sql"),
                &[&email, &username, &nickname, &password],
            )
            .await?;
        Ok(rows.pop().ok_or(AppError::AlreadyExists)?.get(0))
    }

    async fn get<T: Querist>(
        db: &mut T,
        id: Option<&Uuid>,
        email: Option<&str>,
        username: Option<&str>,
    ) -> Result<Option<User>, DbError> {
        use postgres_types::Type;

        let email = email.map(|s| s.to_ascii_lowercase());
        let result = db
            .query_one_typed(
                include_str!("sql/get.sql"),
                &[Type::UUID, Type::TEXT, Type::TEXT],
                &[&id, &email, &username],
            )
            .await;
        inner_map(result, |row| row.get(0))
    }

    pub async fn login<T: Querist>(
        db: &mut T,
        email: Option<&str>,
        username: Option<&str>,
        password: &str,
    ) -> Result<User, AppError> {
        use postgres_types::Type;

        let email = email.map(|s| s.to_ascii_lowercase());
        let row = db
            .query_one_typed(
                include_str!("sql/login.sql"),
                &[Type::TEXT, Type::TEXT, Type::TEXT],
                &[&email, &username, &password],
            )
            .await?
            .ok_or(AppError::NoPermission)?;
        let password_matched = row.get(0);
        if password_matched {
            Ok(row.get(1))
        } else {
            Err(AppError::NoPermission)
        }
    }

    pub async fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<Option<User>, DbError> {
        User::get(db, Some(id), None, None).await
    }

    pub async fn get_by_email<T: Querist>(db: &mut T, email: &str) -> Result<Option<User>, DbError> {
        User::get(db, None, Some(email), None).await
    }

    pub async fn get_by_username<T: Querist>(db: &mut T, username: &str) -> Result<Option<User>, DbError> {
        User::get(db, None, None, Some(username)).await
    }

    pub async fn delete_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<u64, DbError> {
        db.execute(include_str!("sql/delete_by_id.sql"), &[id]).await
    }
}

#[tokio::test]
async fn user_test() -> Result<(), AppError> {
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
    let user = User::get_by_id(&mut trans, &new_user.id).await?.unwrap();
    assert_eq!(user.email, email);
    let user = User::login(&mut trans, Some(email), None, password).await.unwrap();
    assert_eq!(user.nickname, nickname);

    User::delete_by_id(&mut trans, &new_user.id).await.unwrap();

    let all_users = User::all(&mut trans).await.unwrap();
    assert!(all_users.into_iter().find(|u| u.id == user.id).is_none());
    Ok(())
}

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
        Ok(rows.pop().ok_or(AppError::AlreadyExists("User"))?.get(0))
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

    pub async fn deactivated<T: Querist>(db: &mut T, id: &Uuid) -> Result<u64, DbError> {
        db.execute(include_str!("sql/deactivated.sql"), &[id]).await
    }

    pub async fn set<T: Querist>(
        db: &mut T,
        id: &Uuid,
        nickname: Option<String>,
        bio: Option<String>,
        avatar: Option<Uuid>,
    ) -> Result<Option<User>, DbError> {
        let result = db
            .query_one(include_str!("sql/set.sql"), &[id, &nickname, &bio, &avatar])
            .await;
        inner_map(result, |row| row.get(0))
    }
}

#[tokio::test]
async fn user_test() -> Result<(), AppError> {
    use crate::database::Client;
    use crate::media::Media;

    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let db = &mut trans;
    let email = "humura@humura.net";
    let username = "humura";
    let nickname = "Akami Humura";
    let password = "MadokaMadokaSuHaSuHa";
    let new_user = User::create(db, email, username, nickname, password).await.unwrap();
    let user = User::get_by_id(db, &new_user.id).await?.unwrap();
    assert_eq!(user.email, email);
    let user = User::login(db, Some(email), None, password).await.unwrap();
    assert_eq!(user.nickname, nickname);

    let avatar = Media::create(db, "text/plain", user.id, "avatar.jpg", "avatar.jpg", "".to_string(), 0)
        .await?
        .unwrap();
    let new_nickname = "动感超人";
    let bio = "千片万片无数片";
    let user_altered = User::set(
        db,
        &user.id,
        Some(new_nickname.to_string()),
        Some(bio.to_string()),
        Some(avatar.id),
    )
    .await?
    .unwrap();
    assert_eq!(user_altered.nickname, new_nickname);
    assert_eq!(user_altered.bio, bio);
    assert_eq!(user_altered.avatar_id, Some(avatar.id));
    User::deactivated(db, &new_user.id).await.unwrap();

    let all_users = User::all(db).await.unwrap();
    assert!(all_users.into_iter().find(|u| u.id == user.id).is_none());
    Ok(())
}

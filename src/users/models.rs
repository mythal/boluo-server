use postgres_types::FromSql;
use serde::Serialize;
use uuid::Uuid;

use crate::database::Querist;
use crate::error::{DbError, ModelError};
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
    #[serde(with = "crate::date_format")]
    pub joined: chrono::naive::NaiveDateTime,
    #[serde(skip)]
    pub deactivated: bool,
    pub avatar_id: Option<Uuid>,
}

impl User {
    pub async fn all<T: Querist>(db: &mut T) -> Result<Vec<User>, DbError> {
        let rows = db.query_typed(include_str!("sql/all.sql"), &[], &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn register<T: Querist>(
        db: &mut T,
        email: &str,
        username: &str,
        nickname: &str,
        password: &str,
    ) -> Result<User, ModelError> {
        use crate::validators::{DISPLAY_NAME, EMAIL, NAME, PASSWORD};
        let username = username.trim();
        let nickname = nickname.trim();
        let email = email.to_ascii_lowercase();

        EMAIL.run(&email)?;
        DISPLAY_NAME.run(&nickname)?;
        NAME.run(&username)?;
        PASSWORD.run(&password)?;

        let row = db
            .query_exactly_one(
                include_str!("sql/create.sql"),
                &[&email, &username, &nickname, &password],
            )
            .await?;
        Ok(row.get(0))
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

    pub async fn login<T: Querist>(db: &mut T, username: &str, password: &str) -> Result<Option<User>, DbError> {
        use postgres_types::Type;

        let row = db
            .query_one_typed(
                include_str!("sql/login.sql"),
                &[Type::TEXT, Type::TEXT],
                &[&username, &password],
            )
            .await?;

        let result = row.and_then(|row| if row.get(0) { Some(row.get(1)) } else { None });
        Ok(result)
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

    pub async fn edit<T: Querist>(
        db: &mut T,
        id: &Uuid,
        nickname: Option<String>,
        bio: Option<String>,
        avatar: Option<Uuid>,
    ) -> Result<User, ModelError> {
        use crate::validators::{BIO, DISPLAY_NAME};
        let nickname = nickname.as_ref().map(|s| s.trim());
        let bio = bio.as_ref().map(|s| s.trim());
        if let Some(nickname) = nickname {
            DISPLAY_NAME.run(nickname)?;
        }
        if let Some(bio) = bio {
            BIO.run(bio)?;
        }
        db.query_exactly_one(include_str!("sql/edit.sql"), &[id, &nickname, &bio, &avatar])
            .await
            .map_err(Into::into)
            .map(|row| row.get(0))
    }
}

#[tokio::test]
async fn user_test() -> Result<(), crate::error::AppError> {
    use crate::database::Client;
    use crate::media::Media;

    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let db = &mut trans;
    let email = "humura@humura.net";
    let username = "humura";
    let nickname = "Akami Humura";
    let password = "MadokaMadokaSuHaSuHa";
    let new_user = User::register(db, email, username, nickname, password).await.unwrap();
    let user = User::get_by_id(db, &new_user.id).await?.unwrap();
    assert_eq!(user.email, email);
    let user = User::login(db, username, password).await.unwrap().unwrap();
    assert_eq!(user.nickname, nickname);

    let avatar = Media::create(db, "text/plain", user.id, "avatar.jpg", "avatar.jpg", "".to_string(), 0).await?;
    let new_nickname = "动感超人";
    let bio = "千片万片无数片";
    let user_altered = User::edit(
        db,
        &user.id,
        Some(new_nickname.to_string()),
        Some(bio.to_string()),
        Some(avatar.id),
    )
    .await?;
    assert_eq!(user_altered.nickname, new_nickname);
    assert_eq!(user_altered.bio, bio);
    assert_eq!(user_altered.avatar_id, Some(avatar.id));
    User::deactivated(db, &new_user.id).await.unwrap();

    let all_users = User::all(db).await.unwrap();
    assert!(all_users.into_iter().find(|u| u.id == user.id).is_none());
    Ok(())
}

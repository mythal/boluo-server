use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::{CreationError, DbError, FetchError, Querist};
use crate::redis;

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "spaces")]
pub struct Space {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime,
    pub owner_id: Uuid,
    pub is_public: bool,
    pub deleted: bool,
    pub password: String,
    pub language: String,
    pub default_dice_type: String,
}

impl Space {
    pub async fn create<T: Querist>(
        db: &mut T,
        name: &str,
        owner_id: &Uuid,
        password: Option<&str>,
    ) -> Result<Space, CreationError> {
        db.create(include_str!("create_space.sql"), &[], &[&name, owner_id, &password])
            .await
            .map(|row| row.get(0))
    }

    pub async fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<Space, FetchError> {
        use postgres_types::Type;
        db.fetch(include_str!("delete_space.sql"), &[Type::UUID], &[id])
            .await
            .map(|row| row.get(0))
    }

    async fn get<T: Querist>(db: &mut T, id: Option<&Uuid>, name: Option<&str>) -> Result<Space, FetchError> {
        use postgres_types::Type;
        let join_owner = false;
        db.fetch(
            include_str!("fetch_space.sql"),
            &[Type::UUID, Type::TEXT, Type::BOOL],
            &[&id, &name, &join_owner],
        )
        .await
        .map(|row| row.get(0))
    }

    pub async fn all<T: Querist>(db: &mut T) -> Result<Vec<Space>, DbError> {
        let rows = db.query(include_str!("select_spaces.sql"), &[], &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<Space, FetchError> {
        Space::get(db, Some(id), None).await
    }

    pub async fn get_by_name<T: Querist>(db: &mut T, name: &str) -> Result<Space, FetchError> {
        Space::get(db, None, Some(name)).await
    }

    pub async fn is_public<T: Querist>(db: &mut T, id: &Uuid) -> Result<bool, FetchError> {
        let mut cache = redis::get().await;
        let key = redis::make_key(b"spaces", id, b"is_public");
        if let Ok(Some(_)) = cache.get(&*key).await {
            Ok(true)
        } else {
            let space = Space::get_by_id(db, id).await?;
            if space.is_public {
                cache.set(&*key, &[]).await.ok();
            }
            Ok(space.is_public)
        }
    }

    pub async fn delete_caches(id: &Uuid) {
        use crate::redis::make_key;
        let keys = [make_key(b"spaces", id, b"is_public")];
        let mut r = redis::get().await;
        for key in keys.iter() {
            if let Err(e) = r.remove(key).await {
                log::warn!("redis error in delete cache: {}", e);
            }
        }
    }

    pub async fn members<T: Querist>(db: &mut T, space_id: &Uuid) -> Result<Vec<SpaceMember>, DbError> {
        let rows = db
            .query(include_str!("select_space_members.sql"), &[], &[space_id])
            .await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn channels<T: Querist>(db: &mut T, space_id: &Uuid) -> Result<Vec<crate::channels::Channel>, DbError> {
        let rows = db
            .query(include_str!("select_space_channels.sql"), &[], &[space_id])
            .await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }
}

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "space_members")]
pub struct SpaceMember {
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub is_master: bool,
    pub is_admin: bool,
    pub join_date: NaiveDateTime,
}

impl SpaceMember {
    pub async fn set_master<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        space_id: &Uuid,
        is_master: bool,
    ) -> Result<SpaceMember, FetchError> {
        SpaceMember::set(db, user_id, space_id, None, Some(is_master)).await
    }

    pub async fn set_admin<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        space_id: &Uuid,
        is_admin: bool,
    ) -> Result<SpaceMember, FetchError> {
        SpaceMember::set(db, user_id, space_id, Some(is_admin), None).await
    }

    async fn set<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        space_id: &Uuid,
        is_admin: Option<bool>,
        is_master: Option<bool>,
    ) -> Result<SpaceMember, FetchError> {
        db.fetch(
            include_str!("set_space_member.sql"),
            &[],
            &[&is_admin, &is_master, user_id, space_id],
        )
        .await
        .map(|row| row.get(0))
    }

    pub async fn remove_user<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        space_id: &Uuid,
    ) -> Result<SpaceMember, FetchError> {
        db.fetch(include_str!("remove_user_from_space.sql"), &[], &[user_id, space_id])
            .await
            .map(|row| row.get(0))
    }

    pub async fn add_owner<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        space_id: &Uuid,
    ) -> Result<SpaceMember, CreationError> {
        let row = db
            .create(include_str!("add_user_to_space.sql"), &[], &[user_id, space_id, &true])
            .await?;
        Ok(row.get(1))
    }

    pub async fn add_user<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        space_id: &Uuid,
    ) -> Result<SpaceMember, CreationError> {
        let row = db
            .create(include_str!("add_user_to_space.sql"), &[], &[user_id, space_id, &false])
            .await?;
        Ok(row.get(1))
    }

    pub async fn get<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Option<SpaceMember> {
        db.fetch(include_str!("fetch_space_member.sql"), &[], &[user_id, space_id])
            .await
            .map(|row| row.get(0))
            .ok()
    }
}

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "restrained_members")]
pub struct RestrainedMember {
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub blocked: bool,
    pub muted: bool,
    pub restrained_date: NaiveDateTime,
    pub operator_id: Option<Uuid>,
}

impl RestrainedMember {}

#[tokio::test]
async fn space_test() {
    use crate::database::Client;
    use crate::users::User;
    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let db = &mut trans;
    let email = "spaces@mythal.net";
    let username = "space_test";
    let password = "no password";
    let nickname = "Space Test";
    let space_name = "Pure Illusion";
    let user = User::create(db, email, username, nickname, password).await.unwrap();
    let space = Space::create(db, space_name, &user.id, None).await.unwrap();
    let space = Space::get_by_name(db, &space.name).await.unwrap();
    let space = Space::get_by_id(db, &space.id).await.unwrap();
    assert!(Space::is_public(db, &space.id).await.unwrap());
    assert!(Space::is_public(db, &space.id).await.unwrap());
    Space::delete_caches(&space.id).await;
    let spaces = Space::all(db).await.unwrap();
    assert!(spaces.into_iter().find(|s| s.id == space.id).is_some());
    let space = Space::delete(db, &space.id).await.unwrap();
    assert_eq!(space.name, space_name);
    assert_eq!(space.owner_id, user.id);
}

#[tokio::test]
async fn space_member() {
    use crate::database::Client;
    use crate::users::User;

    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let email = "spaces_member@mythal.net";
    let username = "space_member_test";
    let password = "no password";
    let nickname = "Space Member Test User";
    let space_name = "Space Member Test";
    let user = User::create(&mut trans, email, username, nickname, password)
        .await
        .unwrap();
    let space = Space::create(&mut trans, space_name, &user.id, None).await.unwrap();
    let _member = SpaceMember::add_owner(&mut trans, &user.id, &space.id).await.unwrap();
    SpaceMember::set_admin(&mut trans, &user.id, &space.id, true)
        .await
        .unwrap();
    SpaceMember::set_master(&mut trans, &user.id, &space.id, true)
        .await
        .unwrap();
    let member = SpaceMember::remove_user(&mut trans, &user.id, &space.id).await.unwrap();
    assert_eq!(member.user_id, user.id);
    assert_eq!(member.space_id, space.id);
}

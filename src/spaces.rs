use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::{CreationError, FetchError, Querist, query};

#[derive(Debug, Serialize, Deserialize, FromSql)]
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
        db.create(query::CREATE_SPACE.key, &[&name, owner_id, &password])
            .await
            .map(|row| row.get(0))
    }

    pub async fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<Space, FetchError> {
        db.fetch(query::DELETE_SPACE.key, &[id]).await.map(|row| row.get(0))
    }

    async fn get<T: Querist>(db: &mut T, id: Option<&Uuid>, name: Option<&str>) -> Result<Space, FetchError> {
        let join_owner = false;
        db.fetch(query::FETCH_SPACE.key, &[&id, &name, &join_owner])
            .await
            .map(|row| row.get(0))
    }

    pub async fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<Space, FetchError> {
        Space::get(db, Some(id), None).await
    }

    pub async fn get_by_name<T: Querist>(db: &mut T, name: &str) -> Result<Space, FetchError> {
        Space::get(db, None, Some(name)).await
    }
}

#[tokio::test]
async fn space_test() {
    use crate::database::Client;
    use crate::users::User;
    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let email = "spaces@mythal.net";
    let username = "space_test";
    let password = "no password";
    let nickname = "Space Test";
    let space_name = "Pure Illusion";
    let user = User::create(&mut trans, email, username, nickname, password)
        .await
        .unwrap();
    let space = Space::create(&mut trans, space_name, &user.id, None).await.unwrap();
    let space = Space::get_by_name(&mut trans, &space.name).await.unwrap();
    let space = Space::get_by_id(&mut trans, &space.id).await.unwrap();
    let space = Space::delete(&mut trans, &space.id).await.unwrap();
    assert_eq!(space.name, space_name);
    assert_eq!(space.owner_id, user.id);
}

#[derive(Debug, Serialize, Deserialize, FromSql)]
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
        db.fetch(query::SET_SPACE_MEMBER.key, &[&is_admin, &is_master, user_id, space_id])
            .await
            .map(|row| row.get(0))
    }

    async fn remove_user<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Result<SpaceMember, FetchError> {
        db.fetch(query::REMOVE_USER_FROM_SPACE.key, &[user_id, space_id])
            .await
            .map(|row| row.get(0))
    }

    async fn add_user<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Result<SpaceMember, CreationError> {
        db.create(query::ADD_USER_TO_SPACE.key, &[user_id, space_id])
            .await
            .map(|row| row.get(0))
    }
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
    let _member = SpaceMember::add_user(&mut trans, &user.id, &space.id).await.unwrap();
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

#[derive(Debug, Serialize, Deserialize, FromSql)]
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

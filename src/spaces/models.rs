use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::Querist;
use crate::error::{DbError, ModelError};
use crate::spaces::api::SpaceWithMember;
use crate::utils::{inner_map, merge_space};

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
    #[serde(skip)]
    pub deleted: bool,
    #[serde(skip)]
    pub password: String,
    pub language: String,
    pub default_dice_type: String,
}

impl Space {
    pub async fn create<T: Querist>(
        db: &mut T,
        name: String,
        owner_id: &Uuid,
        description: String,
        password: Option<String>,
        default_dice_type: Option<&str>,
    ) -> Result<Space, ModelError> {
        use crate::validators::{DESCRIPTION, DICE, DISPLAY_NAME};
        let name = merge_space(&*name);
        DISPLAY_NAME.run(&name)?;
        if let Some(default_dice_type) = default_dice_type {
            DICE.run(default_dice_type)?;
        }
        DESCRIPTION.run(description.as_str())?;
        let row = db
            .query_exactly_one(
                include_str!("sql/create.sql"),
                &[&name, owner_id, &password, &default_dice_type, &description],
            )
            .await?;
        Ok(row.get(0))
    }

    pub async fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<(), DbError> {
        db.execute(include_str!("sql/delete.sql"), &[id]).await.map(|_| ())
    }

    async fn get<T: Querist>(db: &mut T, id: Option<&Uuid>, name: Option<&str>) -> Result<Option<Space>, DbError> {
        use postgres_types::Type;
        let join_owner = false;
        let result = db
            .query_one_typed(
                include_str!("sql/get.sql"),
                &[Type::UUID, Type::TEXT, Type::BOOL],
                &[&id, &name, &join_owner],
            )
            .await;
        inner_map(result, |row| row.get(0))
    }

    pub async fn all<T: Querist>(db: &mut T) -> Result<Vec<Space>, DbError> {
        let rows = db.query(include_str!("sql/all.sql"), &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<Option<Space>, DbError> {
        Space::get(db, Some(id), None).await
    }

    pub async fn get_by_name<T: Querist>(db: &mut T, name: &str) -> Result<Option<Space>, DbError> {
        Space::get(db, None, Some(name)).await
    }

    pub async fn is_public<T: Querist>(db: &mut T, id: &Uuid) -> Result<Option<bool>, DbError> {
        let row = db.query_one(include_str!("sql/is_public.sql"), &[id]).await?;
        Ok(row.map(|row| row.get(0)))
    }

    pub async fn edit<T: Querist>(
        db: &mut T,
        space_id: Uuid,
        name: Option<String>,
        description: Option<String>,
        default_dice_type: Option<String>,
    ) -> Result<Option<Space>, ModelError> {
        use crate::validators;
        let name = name.as_ref().map(|s| s.trim());
        let description = description.as_ref().map(|s| s.trim());
        if let Some(name) = name {
            validators::DISPLAY_NAME.run(name)?;
        }
        if let Some(description) = description {
            validators::DESCRIPTION.run(description)?;
        }
        if let Some(dice) = default_dice_type.as_ref() {
            validators::DICE.run(dice)?;
        }
        let result = db
            .query_one(
                include_str!("sql/edit.sql"),
                &[&space_id, &name, &description, &default_dice_type],
            )
            .await?;
        Ok(result.map(|row| row.get(0)))
    }

    pub async fn get_by_user<T: Querist>(db: &mut T, user_id: Uuid) -> Result<Vec<SpaceWithMember>, DbError> {
        let rows = db
            .query(include_str!("sql/get_spaces_by_user.sql"), &[&user_id])
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| SpaceWithMember {
                space: row.get(0),
                member: row.get(1),
            })
            .collect())
    }
}

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "space_members")]
pub struct SpaceMember {
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub is_admin: bool,
    pub join_date: NaiveDateTime,
}

impl SpaceMember {
    pub async fn set_admin<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        space_id: &Uuid,
        is_admin: bool,
    ) -> Result<Option<SpaceMember>, DbError> {
        SpaceMember::set(db, user_id, space_id, Some(is_admin)).await
    }

    async fn set<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        space_id: &Uuid,
        is_admin: Option<bool>,
    ) -> Result<Option<SpaceMember>, DbError> {
        let result = db
            .query_one(
                include_str!("sql/set_space_member.sql"),
                &[&is_admin, user_id, space_id],
            )
            .await;
        inner_map(result, |row| row.get(0))
    }

    pub async fn remove_user<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Result<u64, DbError> {
        db.execute(include_str!("sql/remove_user_from_space.sql"), &[user_id, space_id])
            .await
    }

    pub async fn add_admin<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Result<SpaceMember, DbError> {
        db.query_exactly_one(include_str!("sql/add_user_to_space.sql"), &[user_id, space_id, &true])
            .await
            .map(|row| row.get(1))
    }

    pub async fn add_user<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Result<SpaceMember, DbError> {
        db.query_exactly_one(include_str!("sql/add_user_to_space.sql"), &[user_id, space_id, &false])
            .await
            .map(|row| row.get(1))
    }

    pub async fn get<T: Querist>(db: &mut T, user_id: &Uuid, space_id: &Uuid) -> Result<Option<SpaceMember>, DbError> {
        let result = db
            .query_one(include_str!("sql/get_space_member.sql"), &[user_id, space_id])
            .await;
        inner_map(result, |row| row.get(0))
    }

    pub async fn get_by_channel<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
    ) -> Result<Option<SpaceMember>, DbError> {
        let rows = db
            .query(include_str!("sql/get_members_by_channel.sql"), &[user_id, channel_id])
            .await?;
        Ok(rows.into_iter().next().map(|row| row.get(0)))
    }

    pub async fn get_by_space<T: Querist>(db: &mut T, space_id: &Uuid) -> Result<Vec<SpaceMember>, DbError> {
        let rows = db
            .query(include_str!("sql/get_members_by_spaces.sql"), &[space_id])
            .await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
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
async fn space_test() -> Result<(), crate::error::AppError> {
    use crate::database::Client;
    use crate::users::User;
    let mut client = Client::new().await?;
    let mut trans = client.transaction().await?;
    let db = &mut trans;
    let email = "test@mythal.net";
    let username = "test_user";
    let password = "no password";
    let nickname = "Test User";
    let space_name = "Pure Illusion";
    let user = User::register(db, email, username, nickname, password).await.unwrap();
    let space = Space::create(db, space_name.to_string(), &user.id, String::new(), None, None).await?;
    let space = Space::get_by_name(db, &space.name).await?.unwrap();
    let space = Space::get_by_id(db, &space.id).await?.unwrap();
    assert!(Space::is_public(db, &space.id).await?.unwrap());
    let spaces = Space::all(db).await?;
    assert!(spaces.into_iter().find(|s| s.id == space.id).is_some());
    let new_name = "Mythal";
    let description = "some description".to_string();
    let space_edited = Space::edit(db, space.id, Some(new_name.to_string()), Some(description), None)
        .await?
        .unwrap();
    assert_eq!(space_edited.name, new_name);

    let _space_2 = Space::create(db, "学园都市".to_string(), &user.id, String::new(), None, None).await?;
    // let result = Space::edit(db, _space_2.id, Some(new_name.to_string())).await;
    // assert!(if let Err(ModelError::Conflict(_)) = result { true } else { false });
    // members
    SpaceMember::add_admin(db, &user.id, &space.id).await?;
    SpaceMember::get(db, &user.id, &space.id).await.unwrap();
    SpaceMember::set_admin(db, &user.id, &space.id, true).await?;
    let mut members = SpaceMember::get_by_space(db, &space.id).await?;
    let member = members.pop().unwrap();
    assert_eq!(member.user_id, user.id);
    assert_eq!(member.space_id, space.id);

    let joined = Space::get_by_user(db, user.id).await?;
    assert_eq!(joined.len(), 1);
    assert_eq!(joined[0].member.space_id, space.id);

    SpaceMember::remove_user(db, &user.id, &space.id).await?;
    assert!(SpaceMember::get(db, &user.id, &space.id).await?.is_none());

    // delete
    Space::delete(db, &space.id).await?;
    Ok(())
}

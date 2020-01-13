use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::Querist;
use crate::error::{AppError, DbError};
use crate::spaces::SpaceMember;

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "channels")]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub topic: String,
    pub space_id: Uuid,
    pub created: NaiveDateTime,
    pub is_public: bool,
    pub deleted: bool,
}

impl Channel {
    pub async fn create<T: Querist>(
        db: &mut T,
        space_id: &Uuid,
        name: &str,
        is_public: bool,
    ) -> Result<Channel, AppError> {
        let mut rows = db
            .query(include_str!("sql/create_channel.sql"), &[space_id, &name, &is_public])
            .await?;
        Ok(rows.pop().ok_or(AppError::AlreadyExists)?.get(0))
    }

    pub async fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<Channel, AppError> {
        db.fetch(include_str!("sql/fetch_channel.sql"), &[&id])
            .await
            .map(|row| row.get(0))
    }

    pub async fn get_by_space<T: Querist>(db: &mut T, space_id: &Uuid) -> Result<Vec<Channel>, DbError> {
        let rows = db.query(include_str!("sql/get_by_space.sql"), &[space_id]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<u64, DbError> {
        db.execute(include_str!("sql/delete_channel.sql"), &[id]).await
    }
}

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "channel_members")]
pub struct ChannelMember {
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub join_date: NaiveDateTime,
    pub character_name: String,
    pub is_master: bool,
}

impl ChannelMember {
    pub async fn add_user<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
    ) -> Result<ChannelMember, AppError> {
        let mut rows = db
            .query(include_str!("sql/add_user_to_channel.sql"), &[user_id, channel_id, &""])
            .await?;
        Ok(rows
            .pop()
            .ok_or_else(|| unexpected!("the database returned empty result"))?
            .get(1))
    }

    pub async fn get_by_channel<T: Querist>(db: &mut T, channel: &Uuid) -> Result<Vec<ChannelMember>, DbError> {
        let rows = db
            .query(include_str!("sql/get_members_of_channel.sql"), &[channel])
            .await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn is_master<T: Querist>(db: &mut T, user_id: &Uuid, channel_id: &Uuid) -> Result<bool, DbError> {
        let is_master = db
            .query(include_str!("sql/is_master.sql"), &[user_id, channel_id])
            .await?
            .into_iter()
            .next()
            .map(|row| row.get(0))
            .unwrap_or(false);
        Ok(is_master)
    }

    pub async fn get_with_space_member<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
    ) -> Result<Option<(ChannelMember, SpaceMember)>, DbError> {
        let mut rows = db
            .query(include_str!("sql/get_with_space_member.sql"), &[user_id, channel_id])
            .await?;
        Ok(rows.pop().map(|row| (row.get(0), row.get(1))))
    }

    pub async fn get<T: Querist>(db: &mut T, user: &Uuid, channel: &Uuid) -> Option<ChannelMember> {
        db.fetch(include_str!("sql/get_channel_member.sql"), &[user, channel])
            .await
            .map(|row| row.get(0))
            .ok()
    }

    pub async fn remove_user<T: Querist>(db: &mut T, user_id: &Uuid, channel_id: &Uuid) -> Result<u64, DbError> {
        db.execute(include_str!("sql/remove_user_from_channel.sql"), &[user_id, channel_id])
            .await
    }

    pub async fn set_name<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
        character_name: &str,
    ) -> Result<ChannelMember, AppError> {
        db.fetch(
            include_str!("sql/set_name.sql"),
            &[user_id, channel_id, &character_name],
        )
        .await
        .map(|row| row.get(0))
    }


    pub async fn set_master<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
        is_master: bool,
    ) -> Result<ChannelMember, AppError> {
        db.fetch(
            include_str!("sql/set_master.sql"),
            &[user_id, channel_id, &is_master],
        )
            .await
            .map(|row| row.get(0))
    }

    pub async fn remove_by_space<T: Querist>(db: &mut T, user: &Uuid, space: &Uuid) -> Result<(), DbError> {
        db.execute(include_str!("sql/remove_members_by_space.sql"), &[user, space])
            .await?;
        Ok(())
    }
}

#[tokio::test]
async fn channels_test() {
    use crate::database::Client;
    use crate::spaces::Space;
    use crate::users::User;

    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let db = &mut trans;
    let email = "test@mythal.net";
    let username = "test_user";
    let password = "no password";
    let nickname = "Test User";
    let space_name = "Test Space";

    let user = User::create(db, email, username, nickname, password).await.unwrap();
    let space = Space::create(db, space_name, &user.id, None).await.unwrap();
    let channel_name = "Test Channel";
    let channel = Channel::create(db, &space.id, "Test Channel", true).await.unwrap();
    let channel = Channel::get_by_id(db, &channel.id).await.unwrap();
    assert_eq!(channel.space_id, space.id);
    assert_eq!(channel.name, channel_name);

    let channels = Channel::get_by_space(db, &space.id).await.unwrap();
    assert_eq!(channels[0].id, channel.id);

    // members
    SpaceMember::add_owner(db, &user.id, &space.id).await.unwrap();
    let member = ChannelMember::add_user(db, &user.id, &channel.id).await.unwrap();
    let character_name = "Cocona";
    ChannelMember::set_name(db, &member.user_id, &member.channel_id, character_name)
        .await
        .unwrap();
    let member_altered = ChannelMember::get(db, &user.id, &channel.id).await.unwrap();
    assert_eq!(member.join_date, member_altered.join_date);
    assert_eq!(member_altered.character_name, character_name);
    let member_fetched = ChannelMember::get_by_channel(db, &channel.id)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(member_fetched.join_date, member_altered.join_date);
    assert_eq!(member.join_date, member_fetched.join_date);

    ChannelMember::remove_user(db, &user.id, &channel.id).await.unwrap();
    assert_eq!(ChannelMember::get_by_channel(db, &channel.id).await.unwrap().len(), 0);

    ChannelMember::add_user(db, &user.id, &channel.id).await.unwrap();
    let channel_2 = Channel::create(db, &space.id, "Test Channel 2", true).await.unwrap();
    ChannelMember::add_user(db, &user.id, &channel_2.id).await.unwrap();
    ChannelMember::get(db, &user.id, &channel.id)
        .await
        .unwrap();
    ChannelMember::remove_by_space(db, &user.id, &space.id).await.unwrap();
    assert!(ChannelMember::get(db, &user.id, &channel.id).await.is_none());
    assert!(ChannelMember::get(db, &user.id, &channel_2.id).await.is_none());

    // delete
    Channel::delete(db, &channel.id).await.unwrap();
    assert!(Channel::get_by_id(db, &channel.id).await.is_err());
}

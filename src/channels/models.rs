use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::{query, CreationError, FetchError, Querist};

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
    ) -> Result<Channel, CreationError> {
        db.create(query::CREATE_CHANNEL.key, &[space_id, &name, &is_public])
            .await
            .map(|row| row.get(0))
    }

    pub async fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<Channel, FetchError> {
        db.fetch(query::FETCH_CHANNEL.key, &[&id]).await.map(|row| row.get(0))
    }

    pub async fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<Channel, FetchError> {
        db.fetch(query::DELETE_CHANNEL.key, &[id]).await.map(|row| row.get(0))
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
}

impl ChannelMember {
    pub async fn add_user<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
    ) -> Result<ChannelMember, CreationError> {
        db.create(query::ADD_USER_TO_CHANNEL.key, &[user_id, channel_id, &""])
            .await
            .map(|row| row.get(1))
    }

    pub async fn get_by_channel<T: Querist>(
        db: &mut T,
        channel: &Uuid,
    ) -> Result<Vec<ChannelMember>, tokio_postgres::Error> {
        let rows = db.query(query::SELECT_CHANNEL_MEMBERS.key, &[channel]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn get<T: Querist>(db: &mut T, user: &Uuid, channel: &Uuid) -> Option<ChannelMember> {
        db.fetch(query::FETCH_CHANNEL_MEMBER.key, &[user, channel])
            .await
            .map(|row| row.get(0))
            .ok()
    }

    pub async fn remove_user<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
    ) -> Result<ChannelMember, FetchError> {
        db.fetch(query::REMOVE_USER_FROM_CHANNEL.key, &[user_id, channel_id])
            .await
            .map(|row| row.get(0))
    }

    pub async fn set<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
        character_name: &str,
    ) -> Result<ChannelMember, FetchError> {
        db.fetch(query::SET_CHANNEL_MEMBER.key, &[user_id, channel_id, &character_name])
            .await
            .map(|row| row.get(0))
    }
}

#[tokio::test]
async fn channels_test() {
    use crate::database::Client;
    use crate::spaces::Space;
    use crate::users::User;

    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let email = "channels@mythal.net";
    let username = "channel_test";
    let password = "no password";
    let nickname = "Channel Test User";
    let space_name = "Channel Test Space";

    let user = User::create(&mut trans, email, username, nickname, password)
        .await
        .unwrap();
    let space = Space::create(&mut trans, space_name, &user.id, None).await.unwrap();
    let channel_name = "Test Channel";
    let channel = Channel::create(&mut trans, &space.id, "Test Channel", true)
        .await
        .unwrap();
    let channel = Channel::get_by_id(&mut trans, &channel.id).await.unwrap();
    let channel = Channel::delete(&mut trans, &channel.id).await.unwrap();
    assert_eq!(channel.space_id, space.id);
    assert_eq!(channel.name, channel_name);
}

#[tokio::test]
async fn channel_member_test() {
    use crate::database::Client;
    use crate::spaces::Space;
    use crate::users::User;

    let mut client = Client::new().await;
    let mut trans = client.transaction().await.unwrap();
    let db = &mut trans;
    let email = "channels@mythal.net";
    let username = "channel_test";
    let password = "no password";
    let nickname = "Channel Test User";
    let space_name = "Channel Test Space";

    let user = User::create(db, email, username, nickname, password).await.unwrap();
    let space = Space::create(db, space_name, &user.id, None).await.unwrap();
    let channel_name = "Test Channel";
    let channel = Channel::create(db, &space.id, channel_name, true).await.unwrap();
    let member = ChannelMember::add_user(db, &user.id, &channel.id).await.unwrap();
    let character_name = "Cocona";
    ChannelMember::set(db, &member.user_id, &member.channel_id, character_name)
        .await
        .unwrap();
    let member_2 = ChannelMember::get(db, &user.id, &channel.id).await.unwrap();
    assert_eq!(member.join_date, member_2.join_date);
    assert_eq!(member_2.character_name, character_name);
    let member_3 = ChannelMember::get_by_channel(db, &channel.id)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(member_3.join_date, member_2.join_date);
    let member = ChannelMember::remove_user(db, &user.id, &channel.id).await.unwrap();

    assert_eq!(member.character_name, character_name);
}

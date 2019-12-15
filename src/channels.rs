use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::{CreationError, FetchError, Querist, query};

#[derive(Debug, Serialize, Deserialize, FromSql)]
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
    pub fn create<T: Querist>(
        db: &mut T,
        space_id: &Uuid,
        name: &str,
        is_public: bool,
    ) -> Result<Channel, CreationError> {
        db.create(query::CREATE_CHANNEL.key, &[space_id, &name, &is_public])
            .map(|row| row.get(0))
    }

    pub fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<Channel, FetchError> {
        db.fetch(query::FETCH_CHANNEL.key, &[&id]).map(|row| row.get(0))
    }

    pub fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<Channel, FetchError> {
        db.fetch(query::DELETE_CHANNEL.key, &[id]).map(|row| row.get(0))
    }
}

#[test]
fn channels_test() {
    use crate::database::Client;
    use crate::spaces::Space;
    use crate::users::User;

    let mut client = Client::new();
    let mut trans = client.transaction().unwrap();
    let email = "channels@mythal.net";
    let username = "channel_test";
    let password = "no password";
    let nickname = "Channel Test User";
    let space_name = "Channel Test Space";

    let user = User::create(&mut trans, email, username, nickname, password).unwrap();
    let space = Space::create(&mut trans, space_name, &user.id, None).unwrap();
    let channel_name = "Test Channel";
    let channel = Channel::create(&mut trans, &space.id, "Test Channel", true).unwrap();
    let channel = Channel::get_by_id(&mut trans, &channel.id).unwrap();
    let channel = Channel::delete(&mut trans, &channel.id).unwrap();
    assert_eq!(channel.space_id, space.id);
    assert_eq!(channel.name, channel_name);
}

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[postgres(name = "channel_members")]
pub struct ChannelMember {
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub join_date: NaiveDateTime,
    pub character_name: String,
}

impl ChannelMember {
    pub fn add_user<T: Querist>(db: &mut T, user_id: &Uuid, channel_id: &Uuid) -> Result<ChannelMember, CreationError> {
        db.create(query::ADD_USER_TO_CHANNEL.key, &[user_id, channel_id, &""])
            .map(|row| row.get(0))
    }

    pub fn remove_user<T: Querist>(db: &mut T, user_id: &Uuid, channel_id: &Uuid) -> Result<ChannelMember, FetchError> {
        db.fetch(query::REMOVE_USER_FROM_CHANNEL.key, &[user_id, channel_id])
            .map(|row| row.get(0))
    }

    pub fn set<T: Querist>(
        db: &mut T,
        user_id: &Uuid,
        channel_id: &Uuid,
        character_name: &str,
    ) -> Result<ChannelMember, FetchError> {
        db.fetch(query::SET_CHANNEL_MEMBER.key, &[user_id, channel_id, &character_name])
            .map(|row| row.get(0))
    }
}

#[test]
fn channel_member_test() {
    use crate::database::Client;
    use crate::spaces::Space;
    use crate::users::User;

    let mut client = Client::new();
    let mut trans = client.transaction().unwrap();
    let email = "channels@mythal.net";
    let username = "channel_test";
    let password = "no password";
    let nickname = "Channel Test User";
    let space_name = "Channel Test Space";

    let user = User::create(&mut trans, email, username, nickname, password).unwrap();
    let space = Space::create(&mut trans, space_name, &user.id, None).unwrap();
    let channel_name = "Test Channel";
    let channel = Channel::create(&mut trans, &space.id, channel_name, true).unwrap();
    let member = ChannelMember::add_user(&mut trans, &user.id, &channel.id).unwrap();
    let character_name = "Cocona";
    ChannelMember::set(&mut trans, &member.user_id, &member.channel_id, character_name).unwrap();
    let member = ChannelMember::remove_user(&mut trans, &user.id, &channel.id).unwrap();
    assert_eq!(member.character_name, character_name);
}

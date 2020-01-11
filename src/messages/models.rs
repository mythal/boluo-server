use std::collections::HashMap;

use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::database::{CreationError, DbError, FetchError, Querist};
use crate::spaces::SpaceMember;

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "messages")]
pub struct Message {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub channel_id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub name: String,
    pub media_id: Option<Uuid>,
    pub seed: Vec<u8>,
    pub deleted: bool,
    pub in_game: bool,
    pub is_system_message: bool,
    pub is_action: bool,
    pub is_master: bool,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub reaction: HashMap<String, Option<String>>,
    pub crossed_off: bool,
    pub text: String,
    pub whisper_to_users: Option<Vec<Uuid>>,
    pub entities: JsonValue,
    pub metadata: Option<serde_json::Value>,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime,
    pub order_date: NaiveDateTime,
    pub order_offset: i32,
}

impl Message {
    pub async fn get_by_id<T: Querist>(db: &mut T, id: &Uuid) -> Result<Message, FetchError> {
        db.fetch(include_str!("fetch_message.sql"), &[], &[id])
            .await
            .map(|row| row.get(0))
    }

    pub async fn get_by_channel<T: Querist>(db: &mut T, channel_id: &Uuid) -> Result<Vec<Message>, DbError> {
        let rows = db
            .query(include_str!("select_messages.sql"), &[], &[channel_id])
            .await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn create<T: Querist>(
        db: &mut T,
        message_id: Option<&Uuid>,
        channel_id: &Uuid,
        sender_id: &Uuid,
        name: &str,
        text: &str,
        entities: &serde_json::Value,
        in_game: bool,
        is_action: bool,
    ) -> Result<Message, CreationError> {
        db.create(
            include_str!("create_message.sql"),
            &[],
            &[
                &message_id,
                sender_id,
                channel_id,
                &name,
                &text,
                entities,
                &in_game,
                &is_action,
            ],
        )
        .await
        .map(|row| row.get(0))
    }

    pub async fn edit<T: Querist>(
        db: &mut T,
        id: &Uuid,
        text: Option<&str>,
        entities: &Option<JsonValue>,
        in_game: Option<bool>,
        is_action: Option<bool>,
    ) -> Result<Message, FetchError> {
        db.fetch(
            include_str!("edit_message.sql"),
            &[],
            &[id, &text, &entities, &in_game, &is_action],
        )
        .await
        .map(|row| row.get(0))
    }

    pub async fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<Message, FetchError> {
        db.fetch(include_str!("remove_message.sql"), &[], &[id])
            .await
            .map(|row| row.get(0))
    }

    pub async fn get_member<T: Querist>(db: &mut T, id: &Uuid) -> Result<(Message, Option<SpaceMember>), FetchError> {
        db.fetch(include_str!("get_message_and_space_member.sql"), &[], &[id])
            .await
            .map(|row| (row.get(0), row.get(1)))
    }
}

#[tokio::test]
async fn message_test() {
    use crate::channels::Channel;
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
    let space_member = SpaceMember::add_user(db, &user.id, &space.id).await.unwrap();

    let channel_name = "Test Channel";
    let channel = Channel::create(db, &space.id, channel_name, true).await.unwrap();
    let channel_member = ChannelMember::add_user(db, &user.id, &channel.id).await.unwrap();
    let entities = serde_json::Value::Array(vec![]);
    let text = "hello, world";
    let message = Message::create(
        db,
        None,
        &channel.id,
        &user.id,
        &*user.nickname,
        text,
        &entities,
        true,
        false,
    )
    .await
    .unwrap();
    assert_eq!(message.text, text);
    let new_text = "cocona";
    let edited = Message::edit(db, &message.id, Some(new_text), &Some(entities), None, None)
        .await
        .unwrap();
    assert_eq!(edited.text, new_text);
    let message = Message::get_by_id(db, &message.id).await.unwrap();
    let messages = Message::get_by_channel(db, &channel.id).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, message.id);
    Message::delete(db, &message.id).await.unwrap();
    assert!(Message::get_by_id(db, &message.id).await.is_err());
    let (_, _) = Message::get_member(db, &message.id).await.unwrap();
}

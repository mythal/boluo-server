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
    pub async fn get<T: Querist>(db: &mut T, id: &Uuid) -> Result<Message, FetchError> {
        db.fetch(include_str!("sql/get.sql"), &[id]).await.map(|row| row.get(0))
    }

    pub async fn get_by_channel<T: Querist>(db: &mut T, channel_id: &Uuid) -> Result<Vec<Message>, DbError> {
        let rows = db.query(include_str!("sql/get_by_channel.sql"), &[channel_id]).await?;
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
            include_str!("sql/create.sql"),
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
        name: Option<&str>,
        id: &Uuid,
        text: Option<&str>,
        entities: &Option<JsonValue>,
        in_game: Option<bool>,
        is_action: Option<bool>,
    ) -> Result<Message, FetchError> {
        db.fetch(
            include_str!("sql/edit.sql"),
            &[id, &name, &text, &entities, &in_game, &is_action],
        )
        .await
        .map(|row| row.get(0))
    }

    pub async fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<u64, DbError> {
        db.execute(include_str!("sql/delete.sql"), &[id]).await
    }

    pub async fn get_with_space_member<T: Querist>(
        db: &mut T,
        id: &Uuid,
    ) -> Result<(Message, Option<SpaceMember>), FetchError> {
        db.fetch(include_str!("sql/get_with_space_member.sql"), &[id])
            .await
            .map(|row| (row.get(0), row.get(1)))
    }
}

#[tokio::test]
async fn message_test() {
    use crate::channels::{Channel, ChannelMember};
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
    SpaceMember::add_user(db, &user.id, &space.id).await.unwrap();

    let channel_name = "Test Channel";
    let channel = Channel::create(db, &space.id, channel_name, true).await.unwrap();
    ChannelMember::add_user(db, &user.id, &channel.id).await.unwrap();
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
    let edited = Message::edit(db, None, &message.id, Some(new_text), &Some(entities), None, None)
        .await
        .unwrap();
    assert_eq!(edited.text, new_text);
    let message = Message::get(db, &message.id).await.unwrap();
    let messages = Message::get_by_channel(db, &channel.id).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, message.id);
    let (got_message, space_member) = Message::get_with_space_member(db, &message.id).await.unwrap();
    assert_eq!(got_message.id, message.id);
    assert_eq!(space_member.unwrap().space_id, space.id);
    Message::delete(db, &message.id).await.unwrap();
    assert!(Message::get(db, &message.id).await.is_err());
}

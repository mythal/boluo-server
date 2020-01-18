use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::database::Querist;
use crate::error::DbError;
use crate::utils::inner_map;

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
    #[serde(skip)]
    pub deleted: bool,
    pub in_game: bool,
    pub is_action: bool,
    pub is_master: bool,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub folded: bool,
    pub text: String,
    pub whisper_to_users: Option<Vec<Uuid>>,
    pub entities: JsonValue,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime,
    pub order_date: NaiveDateTime,
    pub order_offset: i32,
}

impl Message {
    pub async fn get<T: Querist>(db: &mut T, id: &Uuid, user_id: Option<&Uuid>) -> Result<Option<Message>, DbError> {
        let result = db.query_one(include_str!("sql/get.sql"), &[id, &user_id]).await;
        inner_map(result, |row| row.get(0))
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
        is_master: bool,
        whisper_to: Option<Vec<Uuid>>,
    ) -> Result<Option<Message>, DbError> {
        let result = db
            .query_one(
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
                    &is_master,
                    &whisper_to,
                ],
            )
            .await;
        inner_map(result, |row| row.get(0))
    }

    pub async fn edit<T: Querist>(
        db: &mut T,
        name: Option<&str>,
        id: &Uuid,
        text: Option<&str>,
        entities: &Option<JsonValue>,
        in_game: Option<bool>,
        is_action: Option<bool>,
    ) -> Result<Option<Message>, DbError> {
        let result = db
            .query_one(
                include_str!("sql/edit.sql"),
                &[id, &name, &text, &entities, &in_game, &is_action],
            )
            .await;
        inner_map(result, |row| row.get(0))
    }

    pub async fn delete<T: Querist>(db: &mut T, id: &Uuid) -> Result<u64, DbError> {
        db.execute(include_str!("sql/delete.sql"), &[id]).await
    }
}

#[tokio::test]
async fn message_test() -> Result<(), crate::error::AppError> {
    use crate::channels::{Channel, ChannelMember};
    use crate::database::Client;
    use crate::spaces::Space;
    use crate::spaces::SpaceMember;
    use crate::users::User;

    let mut client = Client::new().await;
    let mut trans = client.transaction().await?;
    let db = &mut trans;
    let email = "test@mythal.net";
    let username = "test_user";
    let password = "no password";
    let nickname = "Test User";
    let space_name = "Test Space";

    let user = User::create(db, email, username, nickname, password).await?;
    let space = Space::create(db, space_name, &user.id, None).await?.unwrap();
    SpaceMember::add_owner(db, &user.id, &space.id).await?;

    let channel_name = "Test Channel";
    let channel = Channel::create(db, &space.id, channel_name, true).await?.unwrap();
    ChannelMember::add_user(db, &user.id, &channel.id).await?;
    ChannelMember::set_master(db, &user.id, &channel.id, true).await?;
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
        true,
        Some(vec![]),
    )
    .await?
    .unwrap();
    assert_eq!(message.text, "");

    let message = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(message.text, text);

    let new_text = "cocona";
    let edited = Message::edit(db, None, &message.id, Some(new_text), &Some(entities), None, None)
        .await?
        .unwrap();
    assert_eq!(edited.text, "");

    let message = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(message.text, new_text);
    ChannelMember::set_master(db, &user.id, &channel.id, false).await?;
    let message = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(message.text, "");

    let messages = Message::get_by_channel(db, &channel.id).await?;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, message.id);
    Message::delete(db, &message.id).await?;
    assert!(Message::get(db, &message.id, Some(&user.id)).await?.is_none());
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub channel_id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub name: String,
    pub media_id: Option<Uuid>,
    pub in_game: bool,
    pub is_action: bool,
    pub is_master: bool,
    pub text: String,
    pub whisper_to_users: Option<Vec<Uuid>>,
    pub entities: JsonValue,
    pub start: NaiveDateTime,
}

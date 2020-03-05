use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::database::Querist;
use crate::error::{DbError, ModelError, ValidationFailed};
use crate::utils::inner_map;
use crate::validators::CHARACTER_NAME;

#[derive(Debug, Serialize, Deserialize, FromSql, Clone)]
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
    #[serde(with = "crate::date_format")]
    pub created: NaiveDateTime,
    #[serde(with = "crate::date_format")]
    pub modified: NaiveDateTime,
    #[serde(with = "crate::date_format")]
    pub order_date: NaiveDateTime,
    pub order_offset: i32,
}

impl Message {
    pub async fn get<T: Querist>(db: &mut T, id: &Uuid, user_id: Option<&Uuid>) -> Result<Option<Message>, DbError> {
        let result = db.query_one(include_str!("sql/get.sql"), &[id, &user_id]).await;
        inner_map(result, |row| row.get(0))
    }

    pub async fn get_by_channel<T: Querist>(db: &mut T, channel_id: &Uuid, before: Option<i64>, limit: i32) -> Result<Vec<Message>, ModelError> {
        use postgres_types::Type;
        if limit > 256 || limit < 1 {
            Err(ValidationFailed("illegal limit range"))?;
        }
        let rows = db.query_typed(include_str!("sql/get_by_channel.sql"), &[Type::UUID, Type::INT8, Type::INT4], &[channel_id, &before, &limit]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn create<T: Querist>(
        db: &mut T,
        message_id: Option<&Uuid>,
        channel_id: &Uuid,
        sender_id: &Uuid,
        default_name: &str,
        mut name: &str,
        text: &str,
        entities: Vec<JsonValue>,
        in_game: bool,
        is_action: bool,
        is_master: bool,
        whisper_to: Option<Vec<Uuid>>,
        order_date: Option<i64>,
    ) -> Result<Message, ModelError> {
        use postgres_types::Type;
        name = name.trim();
        if name.is_empty() {
            name = default_name.trim();
        }
        CHARACTER_NAME.run(name)?;
        if text.is_empty() {
            Err(ValidationFailed("Text is empty."))?;
        }
        let entities = JsonValue::Array(entities);
        let row = db
            .query_exactly_one_typed(
                include_str!("sql/create.sql"),
                &[
                    Type::UUID,
                    Type::UUID,
                    Type::UUID,
                    Type::TEXT,
                    Type::TEXT,
                    Type::JSON,
                    Type::BOOL,
                    Type::BOOL,
                    Type::BOOL,
                    Type::UUID_ARRAY,
                    Type::INT8,
                ],
                &[
                    &message_id,
                    sender_id,
                    channel_id,
                    &name,
                    &text,
                    &entities,
                    &in_game,
                    &is_action,
                    &is_master,
                    &whisper_to,
                    &order_date,
                ],
            )
            .await?;
        Ok(row.get(0))
    }

    pub async fn edit<T: Querist>(
        db: &mut T,
        name: Option<&str>,
        id: &Uuid,
        text: Option<&str>,
        entities: Option<Vec<JsonValue>>,
        in_game: Option<bool>,
        is_action: Option<bool>,
    ) -> Result<Option<Message>, ModelError> {
        let entities = entities.map(JsonValue::Array);
        if let Some(name) = name {
            CHARACTER_NAME.run(name)?;
        }
        let result = db
            .query_one(
                include_str!("sql/edit.sql"),
                &[id, &name, &text, &entities, &in_game, &is_action],
            )
            .await?;
        Ok(result.map(|row| row.get(0)))
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

    let user = User::register(db, email, username, nickname, password).await.unwrap();
    let space = Space::create(db, space_name.to_string(), &user.id, None).await?;
    SpaceMember::add_admin(db, &user.id, &space.id).await?;

    let channel_name = "Test Channel";
    let channel = Channel::create(db, &space.id, channel_name, true).await?;
    ChannelMember::add_user(db, &user.id, &channel.id, "", false).await?;
    ChannelMember::set_master(db, &user.id, &channel.id, true).await?;
    let text = "hello, world";
    let message = Message::create(
        db,
        None,
        &channel.id,
        &user.id,
        "",
        &*user.nickname,
        text,
        vec![],
        true,
        false,
        true,
        Some(vec![]),
        None
    )
    .await?;
    assert_eq!(message.text, "");

    let message = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(message.text, text);

    let new_text = "cocona";
    let edited = Message::edit(db, None, &message.id, Some(new_text), Some(vec![]), None, None)
        .await?
        .unwrap();
    assert_eq!(edited.text, "");

    let message = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(message.text, new_text);
    ChannelMember::set_master(db, &user.id, &channel.id, false).await?;
    let message = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(message.text, "");
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;
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
    pub text: Option<String>,
    pub whisper_to_users: Option<Vec<Uuid>>,
    pub entities: Vec<JsonValue>,
    #[serde(with = "crate::date_format")]
    pub start: NaiveDateTime,
}

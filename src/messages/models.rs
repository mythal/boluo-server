use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::database::{Querist};
use crate::error::{DbError, ModelError, ValidationFailed};
use crate::utils::merge_blank;
use crate::validators::CHARACTER_NAME;

pub fn check_pos(pos: f64) -> Result<(), ValidationFailed> {
    if pos.is_nan() || pos.is_infinite() {
        return Err(ValidationFailed("The wrong floating point value was used for the position value"))
    } else if pos < 0.0 {
        return Err(ValidationFailed("The position value cannot be less than zero"))
    }
    Ok(())
}

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
    pub pos: f64,
}

impl Message {
    pub async fn get<T: Querist>(db: &mut T, id: &Uuid, user_id: Option<&Uuid>) -> Result<Option<Message>, DbError> {
        let result = db
            .query_one(include_str!("sql/get.sql"), &[id, &user_id])
            .await?
            .map(|row| {
                let mut message: Message = row.get(0);
                let should_hide: Option<bool> = row.get(1);
                if should_hide.unwrap_or(true) {
                    message.hide();
                }
                message
            });
        Ok(result)
    }

    pub async fn get_by_channel<T: Querist>(
        db: &mut T,
        channel_id: &Uuid,
        before: Option<f64>,
        limit: i32,
    ) -> Result<Vec<Message>, ModelError> {
        use postgres_types::Type;
        if limit > 256 || limit < 1 {
            return Err(ValidationFailed("illegal limit range").into());
        }
        let rows = db
            .query_typed(
                include_str!("sql/get_by_channel.sql"),
                &[Type::UUID, Type::FLOAT8, Type::INT4],
                &[channel_id, &before, &limit],
            )
            .await?;
        let mut messages: Vec<Message> = rows.into_iter().map(|row| row.get(0)).collect();
        messages.iter_mut().for_each(Message::hide);
        Ok(messages)
    }

    pub async fn export<T: Querist>(
        db: &mut T,
        channel_id: &Uuid,
        hide: bool,
        after: Option<NaiveDateTime>,
    ) -> Result<Vec<Message>, DbError> {
        let rows = db
            .query(include_str!("./sql/export.sql"), &[channel_id, &after])
            .await?;
        let mut messages: Vec<Message> = rows.into_iter().map(|row| row.get(0)).collect();
        if hide {
            messages.iter_mut().for_each(Message::hide);
        }
        Ok(messages)
    }

    pub async fn create<T: Querist>(
        db: &mut T,
        message_id: Option<&Uuid>,
        channel_id: &Uuid,
        sender_id: &Uuid,
        default_name: &str,
        name: &str,
        text: &str,
        entities: Vec<JsonValue>,
        in_game: bool,
        is_action: bool,
        is_master: bool,
        whisper_to: Option<Vec<Uuid>>,
        media_id: Option<Uuid>,
        created: Option<i64>,
        pos: Option<f64>,
    ) -> Result<Message, ModelError> {
        use postgres_types::Type;
        if let Some(pos) = pos {
            check_pos(pos)?;
        }
        let mut name = merge_blank(&*name);
        if name.is_empty() {
            name = default_name.trim().to_string();
        }
        CHARACTER_NAME.run(&name)?;
        if text.is_empty() {
            return Err(ValidationFailed("Text is empty.").into());
        }
        let entities = JsonValue::Array(entities);
        let source = include_str!("sql/create.sql");
        let row = db
            .query_exactly_one_typed(
                source,
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
                    Type::UUID,
                    Type::INT8,
                    Type::FLOAT8,
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
                    &media_id,
                    &created,
                    &pos,
                ],
            )
            .await?;
        let mut message: Message = row.get(0);
        message.hide();
        Ok(message)
    }

    pub fn hide(&mut self) {
        if self.whisper_to_users.is_none() {
            return;
        }
        self.seed = vec![0; 4];
        self.text = String::new();
        self.entities = JsonValue::Array(Vec::new());
    }

    pub async fn move_above<T: Querist>(
        db: &mut T,
        channel_id: &Uuid,
        message_id: &Uuid,
        pos: &f64,
    ) -> Result<Option<Message>, ModelError> {
        use postgres_types::Type;
        check_pos(*pos)?;

        db
            .query_one_typed(include_str!("sql/move_above.sql"), &[Type::UUID, Type::UUID, Type::FLOAT8], &[channel_id, message_id, pos])
            .await
            .map(|row| row.map(|row| row.get(0)))
            .map_err(Into::into)
    }

    pub async fn move_bottom<T: Querist>(
        db: &mut T,
        channel_id: &Uuid,
        message_id: &Uuid,
        pos: &f64,
    ) -> Result<Option<Message>, ModelError> {
        use postgres_types::Type;
        check_pos(*pos)?;

        db
            .query_one_typed(include_str!("sql/move_bottom.sql"), &[Type::UUID, Type::UUID, Type::FLOAT8], &[channel_id, message_id, pos])
            .await
            .map(|row| row.map(|row| row.get(0)))
            .map_err(Into::into)
    }

    pub async fn move_between<T: Querist>(
        db: &mut T,
        id: &Uuid,
        a: &f64,
        b: &f64,
    ) -> Result<Option<Message>, ModelError> {
        use postgres_types::Type;
        check_pos(*a)?;
        check_pos(*b)?;
        if *a == *b {
            return db
                .query_one_typed(include_str!("sql/set_position.sql"), &[Type::UUID, Type::FLOAT8], &[id, a])
                .await
                .map(|row| row.map(|row| row.get(0)))
                .map_err(Into::into);
        }

        db
            .query_one_typed(include_str!("sql/move_between.sql"), &[Type::UUID, Type::FLOAT8, Type::FLOAT8], &[id, a, b])
            .await
            .map(|row| row.map(|row| row.get(0)))
            .map_err(Into::into)
    }
    pub async fn max_pos<T: Querist>(
        db: &mut T,
        channel_id: &Uuid,
    ) -> f64 {
        db
            .query_exactly_one(include_str!("./sql/max_pos.sql"), &[channel_id])
            .await
            .map(|row| row.get(0))
            .unwrap_or(42.0)
    }
    pub async fn edit<T: Querist>(
        db: &mut T,
        name: Option<&str>,
        id: &Uuid,
        text: Option<&str>,
        entities: Option<Vec<JsonValue>>,
        in_game: Option<bool>,
        is_action: Option<bool>,
        folded: Option<bool>,
        media_id: Option<Uuid>,
    ) -> Result<Option<Message>, ModelError> {
        let entities = entities.map(JsonValue::Array);
        let name = name.map(merge_blank);
        if let Some(ref name) = name {
            CHARACTER_NAME.run(name)?;
        }
        let result: Option<Message> = db
            .query_one(
                include_str!("sql/edit.sql"),
                &[id, &name, &text, &entities, &in_game, &is_action, &folded, &media_id],
            )
            .await?
            .map(|row| {
                let mut message: Message = row.get(0);
                message.hide();
                message
            });
        Ok(result)
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
    use crate::utils::timestamp;

    let mut client = Client::new().await?;
    let mut trans = client.transaction().await?;
    let db = &mut trans;
    let email = "test@mythal.net";
    let username = "test_user";
    let password = "no password";
    let nickname = "Test User";
    let space_name = "Test Space";

    let user = User::register(db, email, username, nickname, password).await.unwrap();
    let space = Space::create(db, space_name.to_string(), &user.id, String::new(), None, None).await?;
    SpaceMember::add_admin(db, &user.id, &space.id).await?;

    let channel_name = "Test Channel";
    let channel = Channel::create(db, &space.id, channel_name, true, None).await?;
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
        Some(Uuid::nil()),
        Some(timestamp()),
        None,
    )
    .await?;
    assert_eq!(message.text, "");

    let message = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(message.text, text);

    let new_text = "cocona";
    let edited = Message::edit(
        db,
        None,
        &message.id,
        Some(new_text),
        Some(vec![]),
        None,
        None,
        None,
        None,
    )
    .await?
    .unwrap();
    assert_eq!(edited.text, "");

    let message = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(message.text, new_text);
    ChannelMember::set_master(db, &user.id, &channel.id, false).await?;
    let a = Message::get(db, &message.id, Some(&user.id)).await?.unwrap();
    assert_eq!(a.text, "");
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, a.id);

    let b = Message::create(
        db,
        None,
        &channel.id,
        &user.id,
        "orange",
        &*user.nickname,
        "腰不舒服！",
        vec![],
        true,
        false,
        true,
        None,
        Some(Uuid::nil()),
        Some(timestamp()),
        None,
    )
    .await.unwrap();
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text, b.text);

    let c = Message::create(
        db,
        None,
        &channel.id,
        &user.id,
        "orange",
        &*user.nickname,
        "昨天我打了疫苗，已经是变种人了！",
        vec![],
        true,
        false,
        true,
        None,
        Some(Uuid::nil()),
        Some(timestamp()),
        None,
    )
        .await.unwrap();
    let a = messages[1].pos;
    let b = messages[0].pos;
    Message::move_between(db, &c.id, &a, &b).await.unwrap().unwrap();
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1].id, c.id);
    Message::move_above(db, &c.channel_id, &c.id, &messages[2].pos).await?;
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;
    assert_eq!(messages[2].id, c.id);
    Message::move_bottom(db, &c.channel_id, &c.id, &messages[0].pos).await?;
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;
    assert_eq!(messages[0].id, c.id);
    Ok(())
}

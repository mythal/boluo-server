use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::database::{Querist, Sql};
use crate::error::{DbError, ModelError, ValidationFailed};
use crate::utils::merge_blank;
use crate::validators::CHARACTER_NAME;
use tokio_postgres::Row;

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

#[derive(Debug, Serialize, Deserialize, FromSql, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageOrder {
    pub id: Uuid,
    #[serde(with = "crate::date_format")]
    pub order_date: NaiveDateTime,
    pub order_offset: i32,
}

impl From<Row> for MessageOrder {
    fn from(row: Row) -> MessageOrder {
        MessageOrder {
            id: row.get(0),
            order_date: row.get(1),
            order_offset: row.get(2),
        }
    }
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
        before: Option<i64>,
        limit: i32,
    ) -> Result<Vec<Message>, ModelError> {
        use postgres_types::Type;
        if limit > 256 || limit < 1 {
            return Err(ValidationFailed("illegal limit range").into());
        }
        let rows = db
            .query_typed(
                include_str!("sql/get_by_channel.sql"),
                &[Type::UUID, Type::INT8, Type::INT4],
                &[channel_id, &before, &limit],
            )
            .await?;
        let mut messages: Vec<Message> = rows.into_iter().map(|row| row.get(0)).collect();
        messages.iter_mut().for_each(Message::hide);
        Ok(messages)
    }

    pub async fn export<T: Querist>(db: &mut T, channel_id: &Uuid, hide: bool, after: Option<NaiveDateTime>) -> Result<Vec<Message>, DbError> {
        let rows = db.query(include_str!("./sql/export.sql"), &[channel_id, &after]).await?;
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
        order_date: Option<i64>,
    ) -> Result<Message, ModelError> {
        use postgres_types::Type;
        let mut name = merge_blank(&*name);
        if name.is_empty() {
            name = default_name.trim().to_string();
        }
        CHARACTER_NAME.run(&name)?;
        if text.is_empty() {
            return Err(ValidationFailed("Text is empty.").into());
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
                    Type::UUID,
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
                    &media_id,
                    &order_date,
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
    async fn make_room<T: Querist, U: Into<Sql> + Send>(
        db: &mut T,
        source: U,
        message_id: &Uuid,
        channel_id: &Uuid,
        order_date: &NaiveDateTime,
        order_offset: i32,
        offset_offset: i32,
    ) -> Result<(Vec<Message>, Vec<MessageOrder>), ModelError> {
        db.execute(include_str!("./sql/set_deferred.sql"), &[]).await?;
        let rows = db.query(source, &[channel_id, order_date, &order_offset]).await?;
        let order_list: Vec<MessageOrder> = rows.into_iter().map(Into::into).collect();
        let mut message = Message::set_order(db, message_id, order_date, order_offset + offset_offset).await?;
        message.hide();
        Ok((vec![message], order_list))
    }
    pub async fn move_to_top<T: Querist>(
        db: &mut T,
        id: &Uuid,
        channel_id: &Uuid,
        order_date: &NaiveDateTime,
        order_offset: i32,
    ) -> Result<(Vec<Message>, Vec<MessageOrder>), ModelError> {
        let near_offset: Option<i32> = db
            .query_one(
                include_str!("sql/get_top_near_offset.sql"),
                &[channel_id, order_date, &order_offset],
            )
            .await?
            .map(|row| row.get(0));
        if let Some(near_offset) = near_offset {
            if order_offset - near_offset > 1 {
                let offset = (order_offset - near_offset) >> 1;
                Ok((
                    vec![Message::set_order(db, id, order_date, order_offset - offset).await?],
                    vec![],
                ))
            } else {
                Message::make_room(
                    db,
                    include_str!("sql/make_room_top.sql"),
                    id,
                    channel_id,
                    order_date,
                    order_offset,
                    -8,
                )
                .await
            }
        } else {
            Ok((
                vec![Message::set_order(db, id, order_date, order_offset - 32).await?],
                vec![],
            ))
        }
    }
    pub async fn move_to_bottom<T: Querist>(
        db: &mut T,
        id: &Uuid,
        channel_id: &Uuid,
        order_date: &NaiveDateTime,
        order_offset: i32,
    ) -> Result<(Vec<Message>, Vec<MessageOrder>), ModelError> {
        let near_offset: Option<i32> = db
            .query_one(
                include_str!("sql/get_bottom_near_offset.sql"),
                &[channel_id, order_date, &order_offset],
            )
            .await?
            .map(|row| row.get(0));
        if let Some(near_offset) = near_offset {
            if near_offset - order_offset > 1 {
                let offset = (near_offset - order_offset) >> 1;
                Ok((
                    vec![Message::set_order(db, id, order_date, order_offset + offset).await?],
                    vec![],
                ))
            } else {
                Message::make_room(
                    db,
                    include_str!("sql/make_room_bottom.sql"),
                    id,
                    channel_id,
                    order_date,
                    order_offset,
                    8,
                )
                .await
            }
        } else {
            Ok((
                vec![Message::set_order(db, id, order_date, order_offset + 32).await?],
                vec![],
            ))
        }
    }
    pub async fn set_order<T: Querist>(
        db: &mut T,
        id: &Uuid,
        order_date: &NaiveDateTime,
        order_offset: i32,
    ) -> Result<Message, ModelError> {
        db.query_exactly_one(include_str!("sql/set_order.sql"), &[&id, &order_date, &order_offset])
            .await
            .map(|row| {
                let mut message: Message = row.get(0);
                message.hide();
                message
            })
            .map_err(Into::into)
    }
    pub async fn swap<T: Querist>(db: &mut T, a: &Message, b: &Message) -> Result<Vec<Message>, ModelError> {
        db.execute(include_str!("./sql/set_deferred.sql"), &[]).await?;
        let mut m = Message::set_order(db, &a.id, &b.order_date, b.order_offset).await?;
        let mut n = Message::set_order(db, &b.id, &a.order_date, a.order_offset).await?;
        m.hide();
        n.hide();
        Ok(vec![m, n])
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
    )
    .await?;
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;

    assert_eq!(messages.len(), 2);
    Message::move_to_bottom(db, &b.id, &a.channel_id, &a.order_date, a.order_offset).await?;
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;
    // [b, a]
    assert_eq!(messages[0].id, b.id);
    assert_eq!(messages[1].id, a.id);
    assert_eq!(messages[0].order_date, messages[1].order_date);
    Message::move_to_top(
        db,
        &b.id,
        &messages[1].channel_id,
        &messages[1].order_date,
        messages[1].order_offset,
    )
    .await?;
    let messages = Message::get_by_channel(db, &channel.id, None, 128).await?;
    assert_eq!(messages[0].id, a.id);
    assert_eq!(messages[1].id, b.id);
    Message::export(db, &channel.id, false, None).await.unwrap();
    Message::delete(db, &a.id).await?;
    assert!(Message::get(db, &a.id, Some(&user.id)).await?.is_none());
    Ok(())
}

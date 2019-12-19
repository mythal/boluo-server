use std::collections::HashMap;

use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::{CreationError, Querist, query};

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[postgres(name = "messages")]
pub struct Message {
    id: Uuid,
    sender_id: Uuid,
    channel_id: Uuid,
    parent_message_id: Option<Uuid>,
    name: String,
    media_id: Option<Uuid>,
    seed: Vec<u8>,
    deleted: bool,
    in_game: bool,
    is_system_message: bool,
    is_action: bool,
    is_master: bool,
    pinned: bool,
    tags: Vec<String>,
    reaction: HashMap<String, Option<String>>,
    crossed_off: bool,
    text: String,
    whisper_to_users: Option<Vec<Uuid>>,
    entities: serde_json::Value,
    metadata: Option<serde_json::Value>,
    created: NaiveDateTime,
    modified: NaiveDateTime,
    order_date: NaiveDateTime,
    order_offset: i32,
}

impl Message {
    pub async fn get_messages<T: Querist>(
        db: &mut T,
        channel_id: &Uuid,
    ) -> Result<Vec<Message>, tokio_postgres::Error> {
        let rows = db.query(query::SELECT_MESSAGES.key, &[channel_id]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    pub async fn create_message<T: Querist>(
        db: &mut T,
        channel_id: &Uuid,
        sender_id: &Uuid,
        name: &str,
        text: &str,
    ) -> Result<Message, CreationError> {
        db.create(query::CREATE_MESSAGE.key, &[sender_id, channel_id, &name, &text])
            .await
            .map(|row| row.get(0))
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
    let channel = Channel::create(&mut trans, &space.id, channel_name, true)
        .await
        .unwrap();
    let new_message = Message::create_message(&mut trans, &channel.id, &user.id, &*user.nickname, "hello, world")
        .await
        .unwrap();
    let messages = Message::get_messages(&mut trans, &channel.id).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, new_message.id);
}

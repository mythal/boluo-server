use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::events::context;
use crate::messages::Message;
use crate::error::CacheError;
use crate::{cache, database};
use crate::utils::timestamp;
use std::collections::HashMap;
use crate::events::preview::{Preview, NewPreview};
use crate::events::context::SyncEvent;
use crate::channels::Channel;
use crate::channels::models::Member;
use tokio::spawn;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventQuery {
    pub mailbox: Uuid,
    pub after: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ClientEvent {
    Preview { preview: NewPreview },
}


#[derive(Serialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum EventBody {
    NewMessage { message: Box<Message> },
    #[serde(rename_all = "camelCase")]
    MessageDeleted { message_id: Uuid },
    MessageEdited { message: Box<Message> },
    MessagePreview { preview: Box<Preview> },
    ChannelDeleted,
    ChannelEdited { channel: Channel },
    Members { members: Vec<Member> },
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub mailbox: Uuid,
    pub timestamp: i64,
    pub body: EventBody,
}

impl Event {
    pub fn new_message(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        Event::fire(EventBody::NewMessage { message }, channel_id)
    }

    pub fn message_deleted(channel_id: Uuid, message_id: Uuid) {
        Event::fire(EventBody::MessageDeleted { message_id }, channel_id)
    }

    pub fn message_edited(message: Message) {
        let channel_id = message.channel_id;
        let message = Box::new(message);
        Event::fire(EventBody::MessageEdited { message }, channel_id)
    }

    pub fn channel_deleted(channel_id: Uuid) {
        Event::fire(EventBody::ChannelDeleted, channel_id)
    }

    pub fn message_preview(preview: Preview) {
        let channel_id = preview.channel_id;
        let preview = Box::new(preview);
        spawn(async move {
            if let Err(e) = Event::fire_preview(preview, channel_id).await {
                log::warn!("{}", e);
            }
        });
    }

    pub fn push_members(channel_id: Uuid) {
        spawn(async move {
            if let Err(e) = Event::fire_members(channel_id).await {
                log::warn!("Failed to fetch member list: {}", e);
            }
        });
    }

    pub fn channel_edited(channel: Channel) {
        let channel_id = channel.id;
        Event::fire(EventBody::ChannelEdited { channel }, channel_id);
    }

    pub fn cache_key(mailbox: &Uuid) -> Vec<u8> {
        cache::make_key(b"mailbox", mailbox, b"events")
    }

    pub async fn get_from_cache(mailbox: &Uuid, after: i64) -> Result<Vec<String>, CacheError> {
        let mut cache = cache::get().await;
        let bytes_array = cache.get_after(&*Self::cache_key(mailbox), after + 1).await?;
        let events = bytes_array
            .into_iter()
            .map(|bytes| String::from_utf8(bytes).ok())
            .filter_map(|s| s)
            .collect();
        Ok(events)
    }

    pub async fn wait(mailbox: Uuid) -> Result<SyncEvent, tokio::sync::broadcast::RecvError> {
        context::get_receiver(&mailbox).await.recv().await
    }

    async fn send(mailbox: Uuid, event: SyncEvent) {
        let broadcast_table = context::get_broadcast_table();
        let table = broadcast_table.read().await;
        if let Some(tx) = table.get(&mailbox) {
            tx.send(event).ok();
        }
    }

    async fn fire_members(channel_id: Uuid) -> Result<(), anyhow::Error> {

        let mut db = database::get().await;
        let members = Member::get_by_channel(&mut *db, channel_id).await?;
        drop(db);
        let event = SyncEvent::new(Event{
            mailbox: channel_id,
            body: EventBody::Members { members },
            timestamp: timestamp(),
        });

        Event::send(channel_id, event).await;
        Ok(())
    }

    async fn fire_preview(preview: Box<Preview>, mailbox: Uuid) -> Result<(), anyhow::Error> {
        let sender_id = preview.sender_id;
        let event = SyncEvent::new(Event{
            mailbox,
            body: EventBody::MessagePreview { preview },
            timestamp: timestamp(),
        });

        let cache = context::get_preview_cache();
        let mut mailbox_map = cache.lock().await;
        if let Some(user_map) = mailbox_map.get_mut(&mailbox) {
            user_map.insert(sender_id, event.clone());
        } else {
            let mut user_map = HashMap::new();
            user_map.insert(sender_id, event.clone());
            mailbox_map.insert(mailbox, user_map);
        }
        drop(mailbox_map);

        Event::send(mailbox, event).await;
        Ok(())
    }

    async fn async_fire(body: EventBody, mailbox: Uuid) -> Result<(), anyhow::Error> {
        let event = SyncEvent::new(Event{
            mailbox,
            body,
            timestamp: timestamp(),
        });

        let mut cache = cache::get().await;
        let key = Self::cache_key(&mailbox);
        cache.set_with_time(&*key, event.encoded.as_bytes()).await?;
        drop(cache);


        Event::send(mailbox, event).await;
        Ok(())
    }

    pub fn fire(body: EventBody, mailbox: Uuid) {
        spawn(async move {
            if let Err(e) = Event::async_fire(body, mailbox).await {
                log::warn!("Error on fire event: {}", e);
            }
        });
    }
}
use super::models::{Channel, ChannelMember};
use crate::channels::models::Member;
use crate::spaces::Space;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::users::User;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    pub space_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub character_name: String,
    pub default_dice_type: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub channel_id: Uuid,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub default_dice_type: Option<String>,
    pub default_roll_command: Option<String>,
    #[serde(default)]
    pub grant_masters: Vec<Uuid>,
    #[serde(default)]
    pub remove_masters: Vec<Uuid>,
    pub is_public: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CheckChannelName {
    pub space_id: Uuid,
    pub name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EditMember {
    pub channel_id: Uuid,
    pub character_name: Option<String>,
    pub text_color: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChannelWithRelated {
    pub channel: Channel,
    pub members: Vec<Member>,
    pub space: Space,
    pub color_list: HashMap<Uuid, String>,
    pub heartbeat_map: HashMap<Uuid, i64>,
    pub encoded_events: Vec<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChannelWithMember {
    pub channel: Channel,
    pub member: ChannelMember,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMemberWithUser {
    pub member: ChannelMember,
    pub user: User,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JoinChannel {
    pub channel_id: Uuid,
    #[serde(default)]
    pub character_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddMember {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    #[serde(default)]
    pub character_name: String,
}

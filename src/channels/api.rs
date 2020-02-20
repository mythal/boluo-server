use super::models::{Channel, ChannelMember};
use crate::spaces::Space;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    pub space_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub character_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub channel_id: Uuid,
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
    pub members: Vec<ChannelMember>,
    pub space: Space,
    pub color_list: HashMap<Uuid, String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChannelWithMember {
    pub channel: Channel,
    pub member: ChannelMember,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JoinChannel {
    pub channel_id: Uuid,
    #[serde(default)]
    pub character_name: String,
}

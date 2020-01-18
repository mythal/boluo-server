use super::models::{Channel, ChannelMember};
use crate::spaces::Space;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    pub space_id: Uuid,
    pub name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub channel_id: Uuid,
    pub name: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChannelWithRelated {
    pub channel: Channel,
    pub members: Vec<ChannelMember>,
    pub space: Space,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JoinedChannel {
    pub channel: Channel,
    pub member: ChannelMember,
}

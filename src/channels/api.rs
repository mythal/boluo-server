use super::models::{Channel, ChannelMember};
use crate::spaces::Space;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Create {
    pub space_id: Uuid,
    pub name: String,
}

#[derive(Serialize, Debug)]
pub struct ChannelWithRelated {
    pub channel: Channel,
    pub members: Vec<ChannelMember>,
    pub space: Space,
}

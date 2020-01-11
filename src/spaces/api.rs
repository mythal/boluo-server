use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    pub name: String,
    pub password: Option<String>,
}

impl Create {}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpaceWithRelated {
    pub space: super::Space,
    pub members: Vec<super::SpaceMember>,
    pub channels: Vec<crate::channels::Channel>,
}

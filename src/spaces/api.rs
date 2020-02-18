use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    pub name: String,
    pub password: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub space_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpaceWithRelated {
    pub space: super::Space,
    pub members: Vec<super::SpaceMember>,
    pub channels: Vec<crate::channels::Channel>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpaceWithMember {
    pub space: super::Space,
    pub member: super::SpaceMember,
}

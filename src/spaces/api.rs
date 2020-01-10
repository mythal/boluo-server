use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Create {
    pub name: String,
    pub password: Option<String>,
}

impl Create {}

#[derive(Serialize, Debug)]
pub struct SpaceWithRelated {
    pub space: super::Space,
    pub members: Vec<super::SpaceMember>,
    pub channels: Vec<crate::channels::Channel>,
}

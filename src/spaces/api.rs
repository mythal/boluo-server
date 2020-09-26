use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    pub name: String,
    pub password: Option<String>,
    pub description: String,
    pub default_dice_type: Option<String>,
    pub first_channel_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Join {
    pub space_id: Uuid,
    pub token: Option<Uuid>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Kick {
    pub space_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    pub search: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub space_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub default_dice_type: Option<String>,
    pub explorable: Option<bool>,
    pub is_public: Option<bool>,
    pub allow_spectator: Option<bool>,
    #[serde(default)]
    pub grant_admins: Vec<Uuid>,
    #[serde(default)]
    pub remove_admins: Vec<Uuid>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpaceWithRelated {
    pub space: super::Space,
    pub members: Vec<super::models::SpaceMemberWithUser>,
    pub channels: Vec<crate::channels::Channel>,
}


#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpaceWithMember {
    pub space: super::Space,
    pub member: super::SpaceMember,
}


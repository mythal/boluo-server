use super::User;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::channels::api::ChannelWithMember;
use crate::spaces::api::SpaceWithMember;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryUser {
    pub id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMe {
    pub user: User,
    pub my_channels: Vec<ChannelWithMember>,
    pub my_spaces: Vec<SpaceWithMember>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Register {
    pub email: String,
    pub username: String,
    pub nickname: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub with_token: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginReturn {
    pub me: GetMe,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    pub nickname: Option<String>,
    pub bio: Option<String>,
    pub avatar: Option<Uuid>,
}

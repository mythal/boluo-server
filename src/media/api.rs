use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Upload {
    pub filename: String,
    pub mine_type: String,
}

#[derive(Deserialize)]
pub struct MediaQuery {
    pub filename: Option<String>,
    pub id: Option<Uuid>,
    #[serde(default)]
    pub download: bool,
}

use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Upload {
    pub filename: String,
    pub mime_type: Option<String>,
}

#[derive(Deserialize)]
pub struct MediaQuery {
    pub filename: Option<String>,
    pub id: Option<Uuid>,
    #[serde(default)]
    pub download: bool,
}

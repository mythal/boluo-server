use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "media")]
pub struct Media {
    id: Uuid,
    mine_type: String,
    uploader_id: Uuid,
    filename: String,
    original_filename: String,
    hash: String,
    size: u32,
    description: String,
    created: NaiveDateTime,
}

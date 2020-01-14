use crate::database::Querist;
use crate::error::DbError;
use crate::utils::inner_map;
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

impl Media {
    pub async fn create<T: Querist>(
        db: &mut T,
        mine_type: &str,
        uploader_id: Uuid,
        filename: &str,
        original_filename: &str,
        hash: String,
        size: u32,
    ) -> Result<Option<Media>, DbError> {
        let result = db
            .query_one(
                include_str!("sql/create.sql"),
                &[&mine_type, &uploader_id, &filename, &original_filename, &hash, &size],
            )
            .await;
        inner_map(result, |row| row.get(0))
    }
}

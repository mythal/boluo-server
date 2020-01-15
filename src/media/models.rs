use crate::database::Querist;
use crate::error::DbError;
use crate::utils::inner_map;
use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "media")]
pub struct Media {
    pub id: Uuid,
    pub mine_type: String,
    pub uploader_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub hash: String,
    pub size: u32,
    pub description: String,
    pub created: NaiveDateTime,
}

impl Media {
    pub fn path(filename: &str) -> PathBuf {
        let mut path = PathBuf::from("media");
        path.push(filename);
        path
    }

    pub async fn get_by_id<T: Querist>(db: &mut T, media_id: &Uuid) -> Result<Option<Media>, DbError> {
        let result = db.query_one(include_str!("sql/get_by_id.sql"), &[media_id]).await;
        inner_map(result, |row| row.get(0))
    }

    pub async fn get_by_filename<T: Querist>(db: &mut T, filename: &str) -> Result<Option<Media>, DbError> {
        let result = db
            .query_one(include_str!("sql/get_by_filename.sql"), &[&filename])
            .await;
        inner_map(result, |row| row.get(0))
    }

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

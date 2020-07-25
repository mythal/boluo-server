use crate::database::Querist;
use crate::error::DbError;
use crate::utils::inner_map;
use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub struct MediaFile {
    pub mime_type: String,
    pub filename: String,
    pub original_filename: String,
    pub hash: String,
    pub size: usize,
    pub duplicate: bool,
}

impl MediaFile {
    pub async fn create<T: Querist>(self, db: &mut T, user_id: Uuid) -> Result<Media, DbError> {
        Media::create(db, &*self.mime_type, user_id, &*self.filename, &*self.original_filename, self.hash, self.size as i32).await
    }
}

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[serde(rename_all = "camelCase")]
#[postgres(name = "media")]
pub struct Media {
    pub id: Uuid,
    pub mime_type: String,
    pub uploader_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub hash: String,
    pub size: i32,
    pub description: String,
    #[serde(with = "crate::date_format")]
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
        mime_type: &str,
        uploader_id: Uuid,
        filename: &str,
        original_filename: &str,
        hash: String,
        size: i32,
    ) -> Result<Media, DbError> {
        let row = db
            .query_exactly_one(
                include_str!("sql/create.sql"),
                &[&mime_type, &uploader_id, &filename, &original_filename, &hash, &size],
            )
            .await?;
        Ok(row.get(0))
    }
}

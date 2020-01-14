use super::api::Upload;
use super::models::Media;
use crate::api::{self, parse_query};
use crate::csrf::authenticate;
use crate::database;
use crate::error::AppError;
use crate::utils;
use crc32fast::Hasher;
use futures::StreamExt;
use hyper::{Body, Request};
use once_cell::sync::OnceCell;
use regex::Regex;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::prelude::*;

async fn upload(req: Request<Body>) -> api::AppResult {
    let Upload { filename, mine_type } = parse_query(req.uri())?;
    let session = authenticate(&req).await?;
    let id = utils::id();
    static FILENAME_REPLACE: OnceCell<Regex> = OnceCell::new();
    let filename_replace = FILENAME_REPLACE.get_or_init(|| Regex::new(r#"[/?*:|<>\\]"#).unwrap());
    let origin_filename = filename_replace.replace_all(&filename, "_");
    let new_filename = format!("{}_{}", id, filename);

    if new_filename.len() > 200 {
        return Err(unexpected!("the filename is too long"));
    }
    let mut path = PathBuf::new();
    path.push("media");
    path.push(new_filename);
    let mut file = File::create(&path).await.map_err(unexpected!())?;
    let mut body = req.into_body();
    let mut hasher = Hasher::new();
    let mut size: usize = 0;
    while let Some(bytes) = body.next().await {
        let bytes = bytes?;
        size += bytes.len();
        hasher.update(&bytes);
        file.write_all(&bytes).await.map_err(unexpected!())?;
    }
    let hash = hasher.finalize();

    let filename = path
        .file_name()
        .ok_or_else(|| unexpected!("Failed to get filename from path."))?
        .to_str()
        .ok_or_else(|| unexpected!("Failed to get filename string."))?;

    let mut conn = database::get().await;
    let media = Media::create(
        &mut *conn,
        &*mine_type,
        session.user_id,
        filename,
        &*origin_filename,
        base64::encode(&hash.to_le_bytes()),
        size as u32,
    )
    .await?
    .ok_or(AppError::AlreadyExists)?;
    api::Return::new(&media).build()
}

async fn query(_req: Request<Body>) -> api::AppResult {
    todo!()
}

async fn delete(_req: Request<Body>) -> api::AppResult {
    todo!()
}

pub async fn router(req: Request<Body>, path: &str) -> api::AppResult {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/query", Method::GET) => query(req).await,
        ("/upload", Method::POST) => upload(req).await,
        ("/delete", Method::DELETE) => delete(req).await,
        _ => Err(AppError::missing()),
    }
}

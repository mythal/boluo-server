use super::api::Upload;
use super::models::Media;
use crate::api::{self, parse_query};
use crate::csrf::authenticate;
use crate::database;
use crate::error::AppError;
use crate::media::api::MediaQuery;
use crate::utils;
use futures::StreamExt;
use hyper::header::HeaderValue;
use hyper::{Body, Request, Response};
use once_cell::sync::OnceCell;
use regex::Regex;
use tokio::fs::File;
use tokio::prelude::*;

const MAX_SIZE: usize = 1024 * 1024 * 8;

fn content_disposition(attachment: bool, filename: &str) -> HeaderValue {
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
    let kind = if attachment { "attachment" } else { "inline" };
    const SET: &AsciiSet = &CONTROLS.add(b'"').add(b' ');
    let filename = utf8_percent_encode(filename, SET).to_string();
    HeaderValue::from_str(&*format!("{}; filename*=utf-8''{}", kind, filename)).unwrap()
}

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
    let path = Media::path(&*new_filename);
    let mut file = File::create(&path).await.map_err(unexpected!())?;
    let mut body = req.into_body();
    let mut hasher = blake3::Hasher::new();
    let mut size: usize = 0;
    while let Some(bytes) = body.next().await {
        let bytes = bytes?;
        size += bytes.len();
        if size > MAX_SIZE {
            return Err(AppError::BadRequest(format!(
                "The maximum file size has been exceeded."
            )));
        }
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
        hash.to_hex().to_string(),
        size as u32,
    )
    .await?
    .ok_or(AppError::AlreadyExists)?;
    api::Return::new(&media).build()
}

async fn get(req: Request<Body>) -> api::AppResult {
    use hyper::header;
    let MediaQuery { id, filename, download } = parse_query(req.uri())?;

    let mut conn = database::get().await;
    let db = &mut *conn;
    let mut media: Option<Media> = None;
    if let Some(id) = id {
        media = Some(Media::get_by_id(db, &id).await?.ok_or(AppError::NotFound)?);
    } else if let Some(filename) = filename {
        media = Some(
            Media::get_by_filename(db, &*filename)
                .await?
                .ok_or(AppError::NotFound)?,
        );
    }
    let media = media.ok_or_else(|| AppError::BadRequest(format!("Filename or media id must be specified.")))?;
    let path = Media::path(&*media.filename);

    let mut file = File::open(path).await.map_err(unexpected!())?;
    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf).await.map_err(unexpected!())?;
    let body = Body::from(buf);
    let response = Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&*media.mine_type).map_err(unexpected!())?,
        )
        .header(
            header::CONTENT_DISPOSITION,
            content_disposition(download, &*media.original_filename),
        )
        .header(header::CONTENT_LENGTH, HeaderValue::from(media.size))
        .body(body)
        .map_err(unexpected!())?;
    Ok(response)
}

async fn delete(_req: Request<Body>) -> api::AppResult {
    todo!()
}

pub async fn router(req: Request<Body>, path: &str) -> api::AppResult {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/get", Method::GET) => get(req).await,
        ("/upload", Method::POST) => upload(req).await,
        ("/delete", Method::DELETE) => delete(req).await,
        _ => Err(AppError::missing()),
    }
}

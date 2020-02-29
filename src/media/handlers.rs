use super::api::Upload;
use super::models::Media;
use crate::common::{parse_query, Response, missing, ok_response};
use crate::csrf::authenticate;
use crate::database;
use crate::error::AppError;
use crate::error::ValidationFailed;
use crate::media::api::MediaQuery;
use crate::utils;
use futures::StreamExt;
use hyper::header::{self, HeaderValue};
use hyper::{Body, Request};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::prelude::*;

const MAX_SIZE: usize = 1024 * 1024 * 16;

fn content_disposition(attachment: bool, filename: &str) -> HeaderValue {
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
    let kind = if attachment { "attachment" } else { "inline" };
    const SET: &AsciiSet = &CONTROLS.add(b'"').add(b' ');
    let filename = utf8_percent_encode(filename, SET).to_string();
    HeaderValue::from_str(&*format!("{}; filename*=utf-8''{}", kind, filename)).unwrap()
}

fn filename_sanitizer(filename: String) -> String {
    let filename_replace = regex!(r#"[/?*:|<>\\]"#);
    filename_replace.replace_all(&filename, "_").to_string()
}

fn get_content_type(headers: &header::HeaderMap) -> Option<String> {
    let value = headers.get(header::CONTENT_TYPE)?;
    let str = value.to_str().ok()?;
    Some(str.to_string())
}

fn get_mime_type(mime_type: Option<String>, headers: &header::HeaderMap) -> String {
    mime_type
        .or_else(|| get_content_type(headers))
        .and_then(|s| s.parse().ok())
        .unwrap_or(mime::TEXT_PLAIN)
        .to_string()
}

async fn upload(req: Request<Body>) -> Result<Media, AppError> {
    let Upload { filename, mime_type } = parse_query(req.uri())?;
    let session = authenticate(&req).await?;
    let id = utils::id();

    if filename.len() > 200 {
        Err(ValidationFailed("The filename is too long"))?;
    }

    let mime_type = get_mime_type(mime_type, req.headers());

    let filename = filename_sanitizer(filename);
    let temp_filename = format!("{}_{}", id, filename);

    let path = Media::path(&*temp_filename);
    let mut file = File::create(&path).await?;
    let mut body = req.into_body();
    let mut hasher = blake3::Hasher::new();
    let mut size: usize = 0;
    while let Some(bytes) = body.next().await {
        let bytes = bytes?;
        size += bytes.len();
        if size > MAX_SIZE {
            tokio::fs::remove_file(&*path).await.ok();
            return Err(AppError::BadRequest(format!(
                "The maximum file size has been exceeded."
            )));
        }
        hasher.update(&bytes);
        file.write_all(&bytes).await?;
    }

    let hash = hasher.finalize();
    let hash = hash.to_hex().to_string();
    let ext = path.extension().map(|s| s.to_str()).flatten().unwrap_or("");

    let new_filename = format!("{}.{}", hash, ext);
    let new_path = Media::path(&*new_filename);
    if new_path.exists() {
        tokio::fs::remove_file(path).await?;
    } else {
        tokio::fs::rename(path, new_path).await?;
    }

    let mut conn = database::get().await;
     Media::create(
        &mut *conn,
        &*mime_type,
        session.user_id,
        &*new_filename,
        &*filename,
        hash,
        size as i32,
    )
         .await
         .map_err(Into::into)
}

async fn send_file(path: PathBuf, mut sender: hyper::body::Sender) -> Result<(), anyhow::Error> {
    use bytes::BytesMut;

    let mut file = File::open(path).await?;
    let mut buffer = BytesMut::with_capacity(1024);
    while let Ok(read) = file.read_buf(&mut buffer).await {
        if read == 0 {
            break;
        }
        sender.send_data(buffer.clone().freeze()).await?;
        buffer.clear();
    }
    Ok(())
}

async fn get(req: Request<Body>) -> Result<Response, AppError> {
    let MediaQuery { id, filename, download } = parse_query(req.uri())?;
    let method = req.method().clone();

    let mut conn = database::get().await;
    let db = &mut *conn;
    let mut media: Option<Media> = None;
    if let Some(id) = id {
        media = Some(Media::get_by_id(db, &id).await?.ok_or(AppError::NotFound("media"))?);
    } else if let Some(filename) = filename {
        media = Some(
            Media::get_by_filename(db, &*filename)
                .await?
                .ok_or(AppError::NotFound("media"))?,
        );
    }
    let media = media.ok_or_else(|| AppError::BadRequest(format!("Filename or media id must be specified.")))?;
    let path = Media::path(&*media.filename);

    let body = if method == hyper::Method::HEAD {
        Body::empty()
    } else {
        let (sender, body) = Body::channel();
        tokio::spawn(async move {
            if let Err(e) = send_file(path, sender).await {
                log::error!("Failed to send file: {}", e);
            }
        });
        body
    };

    let response = hyper::Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&*media.mime_type).map_err(unexpected!())?,
        )
        .header(
            header::CONTENT_DISPOSITION,
            content_disposition(download, &*media.original_filename),
        )
        .header(header::ACCEPT_RANGES, HeaderValue::from_static("none"))
        .header(header::CONTENT_LENGTH, HeaderValue::from(media.size))
        .body(body)
        .map_err(unexpected!())?;
    Ok(response)
}

async fn delete(_req: Request<Body>) -> Result<(), AppError> {
    todo!()
}

pub async fn router(req: Request<Body>, path: &str) -> Result<Response, AppError> {
    use hyper::Method;

    match (path, req.method().clone()) {
        ("/get", Method::GET) => get(req).await,
        ("/get", Method::HEAD) => get(req).await,
        ("/upload", Method::POST) => upload(req).await.map(ok_response),
        _ => missing(),
    }
}

use hyper::StatusCode;
pub use redis::RedisError as CacheError;
use std::error::Error;
use thiserror::Error;
pub use tokio_postgres::Error as DbError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("An unexpected database error occurred")]
    Database(#[from] DbError),
    #[error("An unexpected database error occurred")]
    CacheError(#[from] CacheError),
    #[error("Authentication failed")]
    Unauthenticated,
    #[error("Not found record")]
    NotFound,
    #[error("Permission denied")]
    NoPermission,
    #[error("Validation failed: {0}")]
    ValidationFail(String),
    #[error("An unexpected error occurred")]
    Unexpected(anyhow::Error),
    #[error("Wrong request format: {0}")]
    BadRequest(String),
    #[error("Method not allowed")]
    MethodNotAllowed,
    #[error("Record already exists")]
    AlreadyExists,
    #[error("{0}")]
    Custom(String, StatusCode),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        use AppError::*;
        match self {
            Unauthenticated => StatusCode::UNAUTHORIZED,
            NotFound => StatusCode::NOT_FOUND,
            NoPermission => StatusCode::FORBIDDEN,
            ValidationFail(_) | BadRequest(_) => StatusCode::BAD_REQUEST,
            MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            AlreadyExists => StatusCode::CONFLICT,
            Custom(_, code) => code.clone(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn missing() -> AppError {
        AppError::BadRequest(format!("The request was sent with the wrong path or method"))
    }

    pub fn unexpected<E: Error + Send + Sync + 'static>(e: E) -> AppError {
        AppError::Unexpected(e.into())
    }
}

macro_rules! unexpected {
    () => {
        |e| {
            ::log::error!("Unexpected error: [{}][{}]{}", file!(), line!(), e);
            crate::error::AppError::Unexpected(e.into())
        }
    };
    ($msg: expr) => {
        || {
            let msg = $msg.to_string();
            ::log::error!("Unexpected error: [{}][{}]{}", file!(), line!(), msg);
            crate::error::AppError::Unexpected(::anyhow::anyhow!(msg))
        }
    };
}

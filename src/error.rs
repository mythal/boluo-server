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
    #[error("{0} not found")]
    NotFound(&'static str),
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
    #[error("{0} already exists")]
    AlreadyExists(&'static str),
    #[error("An I/O error occurred")]
    HyperError(#[from] hyper::Error),
    #[error("An I/O error occurred")]
    TokioIoError(#[from] tokio::io::Error),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        use AppError::*;
        match self {
            Unauthenticated => StatusCode::UNAUTHORIZED,
            NotFound(_) => StatusCode::NOT_FOUND,
            NoPermission => StatusCode::FORBIDDEN,
            ValidationFail(_) | BadRequest(_) => StatusCode::BAD_REQUEST,
            MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            AlreadyExists(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &'static str {
        use AppError::*;
        match self {
            Unauthenticated => "UNAUTHENTICATED",
            NotFound(_) => "NOT_FOUND",
            NoPermission => "NO_PERMISSION",
            ValidationFail(_) => "VALIDATION_FAIL",
            BadRequest(_) => "BAD_REQUEST",
            MethodNotAllowed => "METHOD_NOT_ALLOWED",
            AlreadyExists(_) => "ALREADY_EXISTS",
            _ => "UNEXPECTED",
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
    ($msg: expr) => {{
        let msg = $msg.to_string();
        ::log::error!("Unexpected error: [{}][{}]{}", file!(), line!(), msg);
        crate::error::AppError::Unexpected(::anyhow::anyhow!(msg))
    }};
}

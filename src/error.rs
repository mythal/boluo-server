use crate::pool::PoolError;
use hyper::StatusCode;
pub use redis::RedisError as CacheError;
use std::error::Error;
use thiserror::Error;
pub use tokio_postgres::Error as DbError;
use std::backtrace::Backtrace;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("An unexpected database error occurred: {source}")]
    Database {
        source: DbError,
        backtrace: Backtrace,
    },
    #[error("An unexpected cache database error occurred: {source}")]
    Cache {
        #[from]
        source: CacheError,
        backtrace: Backtrace,
    },
    #[error("Authentication failed")]
    Unauthenticated,
    #[error("\"{0}\" not found")]
    NotFound(&'static str),
    #[error("Permission denied")]
    NoPermission,
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationFailed),
    #[error("An unexpected error occurred")]
    Unexpected(anyhow::Error),
    #[error("An unexpected serialize error occurred")]
    Serialize(serde_json::Error),
    #[error("Wrong request format: {0}")]
    BadRequest(String),
    #[error("Method not allowed")]
    MethodNotAllowed,
    #[error("\"{0}\" already exists")]
    Conflict(String),
    #[error("An I/O error occurred")]
    Hyper {
        #[from]
        source: hyper::Error,
        backtrace: Backtrace,
    },
    #[error("An I/O error occurred")]
    TokioIo {
        #[from]
        source: tokio::io::Error,
        backtrace: Backtrace,
    },
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        use AppError::*;
        match self {
            Unauthenticated => StatusCode::UNAUTHORIZED,
            NotFound(_) => StatusCode::NOT_FOUND,
            NoPermission => StatusCode::FORBIDDEN,
            Validation(_) | BadRequest(_) => StatusCode::BAD_REQUEST,
            MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            Conflict(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &'static str {
        use AppError::*;
        match self {
            Unauthenticated => "UNAUTHENTICATED",
            NotFound(_) => "NOT_FOUND",
            NoPermission => "NO_PERMISSION",
            Validation(_) => "VALIDATION_FAIL",
            BadRequest(_) => "BAD_REQUEST",
            MethodNotAllowed => "METHOD_NOT_ALLOWED",
            Conflict(_) => "CONFLICT",
            _ => "UNEXPECTED",
        }
    }

    pub fn table(&self) -> Option<String> {
        match self {
            AppError::NotFound(table) => Some(table.to_string()),
            AppError::Conflict(table) => Some(table.to_string()),
            _ => None,
        }
    }

    pub fn missing() -> AppError {
        AppError::NotFound("The request was sent with the wrong path or method")
    }

    pub fn unexpected<E: Error + Send + Sync + 'static>(e: E) -> AppError {
        AppError::Unexpected(e.into())
    }
}

impl From<PoolError> for AppError {
    fn from(e: PoolError) -> AppError {
        AppError::Unexpected(e.into())
    }
}

impl From<DbError> for AppError {
    fn from(e: DbError) -> AppError {
        ModelError::from(e).into()
    }
}

macro_rules! unexpected {
    () => {{
        ::log::error!("Unexpected error: [{}][{}]", file!(), line!());
        crate::error::AppError::Unexpected(::anyhow::anyhow!("Unexpected"))
    }};
    ($msg: expr) => {{
        let msg = $msg.to_string();
        ::log::error!("Unexpected error: [{}][{}]{}", file!(), line!(), msg);
        crate::error::AppError::Unexpected(::anyhow::anyhow!(msg))
    }};
}

macro_rules! error_unexpected {
    () => {
        |e| {
            ::log::error!("Unexpected error: [{}][{}]{}", file!(), line!(), e);
            crate::error::AppError::Unexpected(e.into())
        }
    };
}

#[derive(Error, Debug, Eq, PartialEq)]
#[error("{0}")]
pub struct ValidationFailed(pub &'static str);

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("{0}")]
    Validation(#[from] ValidationFailed),
    #[error("{0}")]
    Database(DbError),
    #[error("{0} already exists")]
    Conflict(String),
}

impl From<ModelError> for AppError {
    fn from(e: ModelError) -> Self {
        match e {
            ModelError::Validation(e) => AppError::Validation(e),
            ModelError::Database(source) => AppError::Database { source, backtrace: Backtrace::capture() },
            ModelError::Conflict(type_) => AppError::Conflict(type_),
        }
    }
}

impl From<DbError> for ModelError {
    fn from(e: DbError) -> Self {
        use tokio_postgres::error::{DbError as DatabaseError, SqlState};

        let db_error: Option<&DatabaseError> = e.source().and_then(<dyn Error>::downcast_ref);
        if let Some(e) = db_error {
            if e.code() == &SqlState::UNIQUE_VIOLATION {
                return ModelError::Conflict(e.table().unwrap_or("Unknown").to_string());
            }
        }
        ModelError::Database(e)
    }
}

pub fn log_error(e: &AppError, source: &str) {
    use crate::error::AppError::*;
    match e {
        NotFound(_) | Conflict(_) => log::debug!("{} - {}", source, e),
        Validation(_) | BadRequest(_) | MethodNotAllowed => {
            log::info!("{} - {}", source, e)
        }
        e => {
            if let Some(backtrace) = e.backtrace() {
                log::error!("{} - {}\n{}", source, e, backtrace);
            } else {
                log::error!("{} - {}\n", source, e);
            }
            sentry::capture_error(e);
        }
    }
}


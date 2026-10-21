use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tracing::{error, warn};

use crate::schemas::MessageResponse;

#[derive(Debug)]
pub enum AppError {
    Database(String),
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Database(e) => {
                error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error occurred".to_string(),
                )
            }
            AppError::NotFound(msg) => {
                warn!("Resource not found: {}", msg);
                (StatusCode::NOT_FOUND, msg)
            }
            AppError::BadRequest(msg) => {
                warn!("Bad request: {}", msg);
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::Internal(msg) => {
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        (status, Json(MessageResponse { message })).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

pub fn db_err<E: std::fmt::Display>(e: E) -> AppError {
    AppError::Database(e.to_string())
}

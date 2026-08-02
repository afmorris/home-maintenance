use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
    #[error("validation error: {0}")]
    #[allow(dead_code)]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    #[allow(dead_code)]
    Forbidden,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("conflict: {0}")]
    #[allow(dead_code)]
    Conflict(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Database(e) => {
                error!("database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "Internal database error".to_string(),
                )
            }
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "Resource not found".to_string(),
            ),
            AppError::Validation(m) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error",
                m.clone(),
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Unauthorized".to_string(),
            ),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Forbidden".to_string()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m.clone()),
            AppError::Internal(m) => {
                error!("internal error: {}", m);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error".to_string(),
                )
            }
            AppError::Conflict(m) => (StatusCode::CONFLICT, "conflict", m.clone()),
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": message,
            }
        }));
        (status, body).into_response()
    }
}

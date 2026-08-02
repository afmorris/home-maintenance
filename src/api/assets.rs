use crate::error::AppError;
use crate::web::AppState;
use axum::Json;
use axum::extract::{Path, State};
use serde_json::json;

pub async fn list_assets(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({"assets": []})))
}

pub async fn create_asset(
    State(_state): State<AppState>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({"id": ""})))
}

pub async fn get_asset(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotFound)
}

pub async fn update_asset(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotFound)
}

pub async fn delete_asset(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotFound)
}

pub async fn placeholder(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({})))
}

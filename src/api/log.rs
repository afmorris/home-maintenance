use crate::error::AppError;
use crate::web::AppState;
use axum::Json;
use axum::extract::State;
use serde_json::json;

pub async fn list_entries(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({"entries": []})))
}

pub async fn create_entry(
    State(_state): State<AppState>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({"id": ""})))
}

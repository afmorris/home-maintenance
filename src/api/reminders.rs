use crate::error::AppError;
use crate::web::AppState;
use axum::Json;
use axum::extract::State;
use serde_json::json;

pub async fn list_reminders(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({"reminders": []})))
}

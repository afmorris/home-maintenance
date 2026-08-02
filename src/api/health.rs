use crate::config::VERSION;
use crate::error::AppError;
use crate::web::AppState;
use axum::Json;
use axum::extract::State;
use serde_json::json;

pub async fn health(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let _row: (i64,) = sqlx::query_as("SELECT 1").fetch_one(&state.db.pool).await?;
    Ok(Json(json!({
        "status": "ok",
        "db_backend": state.db.backend.kind(),
        "version": VERSION,
    })))
}

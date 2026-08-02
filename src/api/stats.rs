use crate::error::AppError;
use crate::web::AppState;
use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct CostQuery {
    group_by: Option<String>,
}

pub async fn costs(
    State(_state): State<AppState>,
    Query(q): Query<CostQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({"group_by": q.group_by})))
}

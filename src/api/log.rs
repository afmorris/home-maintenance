use crate::error::AppError;
use crate::repo::log::{self, LogEntryInput};
use crate::web::AppState;
use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct ListQuery {
    asset_id: Option<String>,
    kind: Option<String>,
    tag: Option<String>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
}

pub async fn list_entries(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = log::list_log_entries(
        &state.db,
        q.asset_id.as_deref(),
        q.kind.as_deref(),
        q.tag.as_deref(),
        q.from.as_deref(),
        q.to.as_deref(),
        q.search.as_deref(),
    )
    .await?;
    Ok(Json(json!({"entries": rows})))
}

#[derive(Deserialize)]
pub struct CreateEntryPayload {
    task_id: Option<String>,
    asset_id: Option<String>,
    kind: String,
    scheduled_date: Option<String>,
    completed_date: String,
    cost_cents: Option<i64>,
    vendor: Option<String>,
    performed_by: Option<String>,
    notes: Option<String>,
}

pub async fn create_entry(
    State(state): State<AppState>,
    Json(payload): Json<CreateEntryPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::now_v7().to_string();
    let input = LogEntryInput {
        task_id: payload.task_id,
        asset_id: payload.asset_id,
        kind: payload.kind,
        scheduled_date: payload.scheduled_date,
        completed_date: payload.completed_date,
        cost_cents: payload.cost_cents,
        vendor: payload.vendor,
        performed_by: payload.performed_by,
        notes: payload.notes,
    };
    let entry = log::create_log_entry(&state.db, &id, input).await?;
    Ok(Json(json!({"id": entry.id, "entry": entry})))
}

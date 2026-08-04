use crate::error::AppError;
use crate::repo::assets;
use crate::repo::log::{self, LogEntryInput};
use crate::repo::tasks;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::response::Redirect;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LogQuery {
    asset_id: Option<String>,
    kind: Option<String>,
    tag: Option<String>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
}

#[derive(Template)]
#[template(path = "log.html")]
pub struct LogListView {
    pub title: String,
    pub entries: Vec<log::LogEntry>,
    pub assets: Vec<assets::Asset>,
    pub query: LogQuery,
}

pub async fn list_log(
    State(state): State<AppState>,
    Query(q): Query<LogQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let entries = log::list_log_entries(
        &state.db,
        q.asset_id.as_deref(),
        q.kind.as_deref(),
        q.tag.as_deref(),
        q.from.as_deref(),
        q.to.as_deref(),
        q.search.as_deref(),
    )
    .await?;
    let assets = assets::list_assets(&state.db).await?;

    LogListView {
        title: "Log".to_string(),
        entries,
        assets,
        query: q,
    }
    .render_response()
}

#[derive(Deserialize)]
pub struct LogFormPayload {
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
    axum::extract::Form(payload): axum::extract::Form<LogFormPayload>,
) -> Result<Redirect, AppError> {
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
    log::create_log_entry(&state.db, &id, input).await?;
    Ok(Redirect::to("/log"))
}

#[derive(Template)]
#[template(path = "log_form.html")]
pub struct LogFormView {
    pub title: String,
    pub assets: Vec<assets::Asset>,
    pub tasks: Vec<tasks::Task>,
}

pub async fn new_log_form(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let assets = assets::list_assets(&state.db).await?;
    let tasks = tasks::list_tasks(&state.db).await?;
    LogFormView {
        title: "New Log Entry".to_string(),
        assets,
        tasks,
    }
    .render_response()
}

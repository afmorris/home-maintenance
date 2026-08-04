use crate::config::CONFIG;
use crate::error::AppError;
use crate::repo::assets;
use crate::repo::tasks::get_task;
use crate::repo::tasks::{self, TaskInput};
use crate::util::today_in_tz;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "tasks.html")]
pub struct TaskListView {
    pub title: String,
    pub tasks: Vec<TaskRow>,
}

pub struct TaskRow {
    pub id: String,
    pub asset_name: Option<String>,
    pub name: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub schedule: String,
    pub due_date: Option<String>,
}

#[derive(Deserialize)]
pub struct TaskListQuery {
    asset_id: Option<String>,
}

pub async fn list_tasks(
    State(state): State<AppState>,
    Query(q): Query<TaskListQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let assets = assets::list_assets(&state.db).await?;
    let asset_names: std::collections::HashMap<String, String> =
        assets.into_iter().map(|a| (a.id, a.name)).collect();

    let all_tasks = tasks::list_tasks(&state.db).await?;
    let reminders = crate::repo::reminders::list_reminders(&state.db, None, None, None).await?;
    let reminder_map: std::collections::HashMap<String, crate::repo::reminders::Reminder> =
        reminders
            .into_iter()
            .map(|r| (r.task_id.clone(), r))
            .collect();

    let rows: Vec<TaskRow> = all_tasks
        .into_iter()
        .filter(|t| {
            q.asset_id
                .as_ref()
                .map(|aid| t.asset_id.as_ref() == Some(aid))
                .unwrap_or(true)
        })
        .map(|t| TaskRow {
            id: t.id.clone(),
            asset_name: t
                .asset_id
                .as_ref()
                .and_then(|id| asset_names.get(id).cloned()),
            name: t.name,
            description: t.description,
            schedule: crate::domain::schedule::describe_schedule(
                &t.schedule_mode,
                t.interval_value,
                t.interval_unit.as_deref(),
                t.season_anchor.as_deref(),
                t.fixed_interval_years,
            ),
            due_date: reminder_map.get(&t.id).map(|r| r.due_date.clone()),
        })
        .collect();

    TaskListView {
        title: "Tasks".to_string(),
        tasks: rows,
    }
    .render_response()
}

#[derive(Template)]
#[template(path = "task_form.html")]
pub struct TaskFormView {
    pub title: String,
    pub assets: Vec<assets::Asset>,
    pub task: Option<tasks::Task>,
    pub is_edit: bool,
}

pub async fn new_task_form(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let assets = assets::list_assets(&state.db).await?;
    TaskFormView {
        title: "New Task".to_string(),
        assets,
        task: None,
        is_edit: false,
    }
    .render_response()
}

pub async fn edit_task_form(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let task = get_task(&state.db, &id).await?;
    let assets = assets::list_assets(&state.db).await?;
    TaskFormView {
        title: format!("Edit {}", task.name),
        assets,
        task: Some(task),
        is_edit: true,
    }
    .render_response()
}

#[derive(Deserialize)]
pub struct TaskFormPayload {
    asset_id: Option<String>,
    name: String,
    description: Option<String>,
    schedule_mode: String,
    interval_value: Option<i64>,
    interval_unit: Option<String>,
    season_anchor: Option<String>,
    fixed_interval_years: Option<i64>,
    estimated_minutes: Option<i64>,
    last_done_date: Option<String>,
}

pub async fn create_task(
    State(state): State<AppState>,
    axum::extract::Form(payload): axum::extract::Form<TaskFormPayload>,
) -> Result<Redirect, AppError> {
    let today = today_in_tz(&CONFIG.app_timezone);
    let id = uuid::Uuid::now_v7().to_string();
    let input = task_input_from_payload(&payload);
    let last_done = payload
        .last_done_date
        .and_then(|s| s.parse::<chrono::NaiveDate>().ok());

    tasks::create_task(&state.db, &id, input, today, last_done).await?;
    Ok(Redirect::to("/tasks"))
}

pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Form(payload): axum::extract::Form<TaskFormPayload>,
) -> Result<Redirect, AppError> {
    let today = today_in_tz(&CONFIG.app_timezone);
    let input = task_input_from_payload(&payload);
    let last_done = payload
        .last_done_date
        .and_then(|s| s.parse::<chrono::NaiveDate>().ok());

    tasks::update_task(&state.db, &id, input, today, last_done).await?;
    Ok(Redirect::to("/tasks"))
}

fn task_input_from_payload(payload: &TaskFormPayload) -> TaskInput {
    TaskInput {
        asset_id: payload.asset_id.clone(),
        name: payload.name.clone(),
        description: payload.description.clone(),
        schedule_mode: payload.schedule_mode.clone(),
        interval_value: payload.interval_value,
        interval_unit: payload.interval_unit.clone(),
        season_anchor: payload.season_anchor.clone(),
        fixed_interval_years: payload.fixed_interval_years,
        estimated_minutes: payload.estimated_minutes,
    }
}

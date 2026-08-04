use crate::config::CONFIG;
use crate::domain::recurrence::ReminderStatus;
use crate::error::AppError;
use crate::repo::log::LogEntry;
use crate::repo::reminders::{complete_task_transaction, snooze_reminder};
use crate::repo::tasks::{self, TaskInput};
use crate::util::today_in_tz;
use crate::web::AppState;
use axum::Json;
use axum::extract::{Path, State};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;

pub async fn list_tasks(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = tasks::list_tasks(&state.db).await?;
    Ok(Json(json!({"tasks": rows})))
}

#[derive(Deserialize)]
pub struct CreateTaskPayload {
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

pub async fn create_task_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_task_payload(&payload)?;

    let id = uuid::Uuid::now_v7().to_string();
    let input = task_input_from_payload(&payload);
    let today = today_in_tz(&CONFIG.app_timezone);
    let last_done = payload
        .last_done_date
        .and_then(|s| s.parse::<NaiveDate>().ok());

    let task = tasks::create_task(&state.db, &id, input, today, last_done).await?;
    Ok(Json(json!({"id": task.id, "task": task})))
}

#[derive(Deserialize)]
pub struct UpdateTaskPayload {
    asset_id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    schedule_mode: Option<String>,
    interval_value: Option<i64>,
    interval_unit: Option<String>,
    season_anchor: Option<String>,
    fixed_interval_years: Option<i64>,
    estimated_minutes: Option<i64>,
    last_done_date: Option<String>,
}

pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTaskPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let input = TaskInput {
        asset_id: payload.asset_id,
        name: payload.name.unwrap_or_default(),
        description: payload.description,
        schedule_mode: payload.schedule_mode.unwrap_or_default(),
        interval_value: payload.interval_value,
        interval_unit: payload.interval_unit,
        season_anchor: payload.season_anchor,
        fixed_interval_years: payload.fixed_interval_years,
        estimated_minutes: payload.estimated_minutes,
    };

    let today = today_in_tz(&CONFIG.app_timezone);
    let last_done = payload
        .last_done_date
        .and_then(|s| s.parse::<NaiveDate>().ok());

    let task = tasks::update_task(&state.db, &id, input, today, last_done).await?;
    Ok(Json(json!({"id": task.id, "task": task})))
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let task = tasks::get_task(&state.db, &id).await?;
    Ok(Json(json!({"task": task})))
}

pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    tasks::delete_task(&state.db, &id).await?;
    Ok(Json(json!({"deleted": true})))
}

fn validate_task_payload(payload: &CreateTaskPayload) -> Result<(), AppError> {
    match payload.schedule_mode.as_str() {
        "floating" => {
            if payload.interval_value.is_none() || payload.interval_unit.is_none() {
                return Err(AppError::BadRequest(
                    "floating schedule requires interval_value and interval_unit".to_string(),
                ));
            }
        }
        "fixed" => {
            if payload.season_anchor.is_none() {
                return Err(AppError::BadRequest(
                    "fixed schedule requires season_anchor".to_string(),
                ));
            }
        }
        _ => {
            return Err(AppError::BadRequest(format!(
                "invalid schedule_mode: {}",
                payload.schedule_mode
            )));
        }
    }
    Ok(())
}

fn task_input_from_payload(payload: &CreateTaskPayload) -> TaskInput {
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

#[derive(Deserialize)]
pub struct CompletePayload {
    completed_date: Option<String>,
    cost_cents: Option<i64>,
    vendor: Option<String>,
    performed_by: Option<String>,
    notes: Option<String>,
}

pub async fn complete_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(payload): Json<CompletePayload>,
) -> Result<Json<LogEntry>, AppError> {
    let completed_date = payload
        .completed_date
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| today_in_tz(&CONFIG.app_timezone));

    let log_id = uuid::Uuid::now_v7().to_string();

    let entry = complete_task_transaction(
        &state.db,
        &log_id,
        &task_id,
        None,
        "service",
        completed_date,
        payload.cost_cents,
        payload.vendor.as_deref(),
        payload.performed_by.as_deref(),
        payload.notes.as_deref(),
    )
    .await?;

    Ok(Json(entry))
}

#[derive(Deserialize)]
pub struct SnoozePayload {
    days: Option<i64>,
    until: Option<String>,
}

pub async fn snooze_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(payload): Json<SnoozePayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let today = today_in_tz(&CONFIG.app_timezone);

    let snoozed_until = if let Some(until) = payload.until {
        until
            .parse::<NaiveDate>()
            .map_err(|e| AppError::BadRequest(format!("invalid until date: {e}")))?
            .to_string()
    } else {
        let days = payload.days.unwrap_or(7).max(1);
        today
            .checked_add_days(chrono::Days::new(days as u64))
            .ok_or_else(|| AppError::BadRequest("invalid snooze duration".to_string()))?
            .to_string()
    };

    let reminder = snooze_reminder(&state.db, &task_id, &snoozed_until).await?;
    let due = reminder.due_date.parse().unwrap_or(today);
    let snooze = reminder.snoozed_until.as_ref().and_then(|s| s.parse().ok());
    let status = ReminderStatus::as_str(&derive_status(today, due, snooze)).to_string();

    Ok(Json(json!({
        "id": reminder.id,
        "task_id": reminder.task_id,
        "due_date": reminder.due_date,
        "snoozed_until": reminder.snoozed_until,
        "status": status,
    })))
}

fn derive_status(today: NaiveDate, due: NaiveDate, snooze: Option<NaiveDate>) -> ReminderStatus {
    if let Some(snooze) = snooze
        && today <= snooze
    {
        return ReminderStatus::Upcoming;
    }
    if due < today {
        ReminderStatus::Overdue
    } else if due.signed_duration_since(today).num_days() <= 7 {
        ReminderStatus::Due
    } else {
        ReminderStatus::Upcoming
    }
}

use crate::config::CONFIG;
use crate::domain::recurrence::ReminderStatus;
use crate::error::AppError;
use crate::repo::log::LogEntry;
use crate::repo::reminders::{complete_task_transaction, snooze_reminder};
use crate::util::today_in_tz;
use crate::web::AppState;
use axum::Json;
use axum::extract::{Path, State};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;

pub async fn list_tasks(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({"tasks": []})))
}

pub async fn create_task(
    State(_state): State<AppState>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({"id": ""})))
}

pub async fn get_task(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotFound)
}

pub async fn update_task(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotFound)
}

pub async fn delete_task(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotFound)
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

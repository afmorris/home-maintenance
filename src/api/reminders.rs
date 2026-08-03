use crate::config::CONFIG;
use crate::domain::recurrence::derive_status;
use crate::error::AppError;
use crate::repo::reminders::{ReminderWithTask, list_reminders_with_tasks};
use crate::util::today_in_tz;
use crate::web::AppState;
use axum::Json;
use axum::extract::State;
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
struct ReminderJson {
    id: String,
    task_id: String,
    task_name: String,
    asset_name: Option<String>,
    due_date: String,
    status: String,
    snoozed_until: Option<String>,
}

impl ReminderJson {
    fn from_row(today: NaiveDate, row: ReminderWithTask) -> Self {
        let due = row.due_date.parse().unwrap_or(today);
        let snooze = row.snoozed_until.as_ref().and_then(|s| s.parse().ok());
        let status = derive_status(today, due, snooze).as_str().to_string();
        Self {
            id: row.id,
            task_id: row.task_id,
            task_name: row.task_name,
            asset_name: row.asset_name,
            due_date: row.due_date,
            status,
            snoozed_until: row.snoozed_until,
        }
    }
}

pub async fn list_reminders(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let today = today_in_tz(&CONFIG.app_timezone);
    let rows = list_reminders_with_tasks(&state.db, None).await?;

    let mut overdue_count = 0usize;
    let mut due_count = 0usize;
    let mut upcoming_count = 0usize;

    let reminders: Vec<ReminderJson> = rows
        .into_iter()
        .map(|r| {
            let json = ReminderJson::from_row(today, r);
            match json.status.as_str() {
                "overdue" => overdue_count += 1,
                "due" => due_count += 1,
                _ => upcoming_count += 1,
            }
            json
        })
        .collect();

    Ok(Json(json!({
        "reminders": reminders,
        "overdue_count": overdue_count,
        "due_count": due_count,
        "upcoming_count": upcoming_count,
    })))
}

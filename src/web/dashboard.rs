use crate::config::CONFIG;
use crate::domain::recurrence::derive_status;
use crate::error::AppError;
use crate::repo::log::{LogEntryWithNames, recent_log_entries};
use crate::repo::reminders::{ReminderWithTask, list_reminders_with_tasks};
use crate::util::today_in_tz;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::State;
use chrono::NaiveDate;

fn render_card(r: &ReminderView) -> String {
    ReminderCard {
        _id: r.id.clone(),
        task_id: r.task_id.clone(),
        task_name: r.task_name.clone(),
        asset_name: r.asset_name.clone(),
        due_date: r.due_date.clone(),
        _status: r.status.clone(),
        _snoozed_until: r.snoozed_until.clone(),
    }
    .render()
    .unwrap_or_default()
}

#[derive(Template)]
#[template(path = "reminder_card.html")]
pub struct ReminderCard {
    pub _id: String,
    pub task_id: String,
    pub task_name: String,
    pub asset_name: Option<String>,
    pub due_date: String,
    pub _status: String,
    pub _snoozed_until: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardView {
    pub title: String,
    pub overdue: Vec<ReminderView>,
    pub due: Vec<ReminderView>,
    pub upcoming: Vec<ReminderView>,
    pub recent_log: Vec<LogEntryWithNames>,
    pub counts: Counts,
}

pub struct Counts {
    pub overdue: usize,
    pub due: usize,
    pub upcoming: usize,
}

pub struct ReminderView {
    pub id: String,
    pub task_id: String,
    pub task_name: String,
    pub asset_name: Option<String>,
    pub due_date: String,
    pub status: String,
    pub snoozed_until: Option<String>,
}

impl ReminderView {
    pub fn render(&self) -> String {
        render_card(self)
    }

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

pub async fn index(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let today = today_in_tz(&CONFIG.app_timezone);

    let rows = list_reminders_with_tasks(&state.db, None).await?;
    let recent_log = recent_log_entries(&state.db, 20).await?;

    let mut overdue = Vec::new();
    let mut due = Vec::new();
    let mut upcoming = Vec::new();

    for row in rows {
        let view = ReminderView::from_row(today, row);
        match view.status.as_str() {
            "overdue" => overdue.push(view),
            "due" => due.push(view),
            _ => upcoming.push(view),
        }
    }

    let upcoming_cutoff = today
        .checked_add_days(chrono::Days::new(30))
        .unwrap_or(today);
    upcoming.retain(|r| {
        r.due_date
            .parse::<NaiveDate>()
            .map(|d| d <= upcoming_cutoff)
            .unwrap_or(false)
    });

    let counts = Counts {
        overdue: overdue.len(),
        due: due.len(),
        upcoming: upcoming.len(),
    };

    DashboardView {
        title: format!("Home Maintenance — {}", CONFIG.app_timezone),
        overdue,
        due,
        upcoming,
        recent_log,
        counts,
    }
    .render_response()
}

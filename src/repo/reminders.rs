#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
use crate::db::{Db, query, query_as};
use crate::domain::recurrence::{next_due_fixed, next_due_floating};
use crate::error::AppError;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct Reminder {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub task_id: String,
    pub due_date: String,
    pub snoozed_until: Option<String>,
    pub last_notified_at: Option<String>,
}

#[derive(Debug, Default)]
pub struct ReminderInput {
    pub due_date: String,
    pub snoozed_until: Option<String>,
    pub task_id: String,
}

/// Flat reminder row with joined task and asset names, intended for dashboard
/// rendering. The status string is derived in Rust so the same logic works on
/// both Postgres and SQLite; the SQL projection avoids dialect-specific date
/// functions except for optional SQLite-side filtering in `list_reminders`.
#[derive(Debug, FromRow)]
pub struct ReminderWithTask {
    pub id: String,
    pub task_id: String,
    pub task_name: String,
    pub asset_id: Option<String>,
    pub asset_name: Option<String>,
    pub due_date: String,
    pub snoozed_until: Option<String>,
}

pub async fn list_reminders_with_tasks(
    db: &Db,
    limit: Option<i64>,
) -> Result<Vec<ReminderWithTask>, AppError> {
    let mut sql = String::from(
        "SELECT
            r.id,
            r.task_id,
            t.name AS task_name,
            t.asset_id,
            a.name AS asset_name,
            r.due_date,
            r.snoozed_until
         FROM reminders r
         JOIN tasks t ON t.id = r.task_id
         LEFT JOIN assets a ON a.id = t.asset_id
         WHERE t.active = 1
         ORDER BY r.due_date",
    );

    if limit.is_some() {
        sql.push_str(" LIMIT $L");
    }

    let mut p_sql = sql;
    if limit.is_some() {
        p_sql = p_sql.replace("$L", "$1");
    }

    let static_sql: &'static str = Box::leak(p_sql.into_boxed_str());
    let mut q = query_as::<ReminderWithTask>(static_sql, db);

    if let Some(l) = limit {
        q = q.bind(l);
    }

    let rows = q.fetch_all(&db.pool).await?;
    Ok(rows)
}

pub async fn list_reminders(
    db: &Db,
    status: Option<&str>,
    asset_id: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<Reminder>, AppError> {
    let mut sql = String::from(
        "SELECT r.* FROM reminders r
         JOIN tasks t ON t.id = r.task_id
         WHERE t.active = 1",
    );

    if asset_id.is_some() {
        sql.push_str(" AND t.asset_id = $P");
    }

    if status.is_some() {
        sql.push_str(
            " AND (
            CASE
                WHEN r.snoozed_until IS NOT NULL AND date(r.snoozed_until) >= date('now') THEN 'upcoming'
                WHEN date(r.due_date) < date('now') THEN 'overdue'
                WHEN julianday(r.due_date) - julianday('now') <= 7 THEN 'due'
                ELSE 'upcoming'
            END
        ) = $S",
        );
    }

    sql.push_str(" ORDER BY r.due_date");

    if limit.is_some() {
        sql.push_str(" LIMIT $L");
    }

    let mut next = 1usize;
    let mut p_sql = sql;
    if asset_id.is_some() {
        p_sql = p_sql.replace("$P", &format!("${}", next));
        next += 1;
    }
    if status.is_some() {
        p_sql = p_sql.replace("$S", &format!("${}", next));
        next += 1;
    }
    if limit.is_some() {
        p_sql = p_sql.replace("$L", &format!("${}", next));
    }

    let static_sql: &'static str = Box::leak(p_sql.into_boxed_str());
    let mut q = query_as::<Reminder>(static_sql, db);

    if let Some(aid) = asset_id {
        q = q.bind(aid);
    }
    if let Some(st) = status {
        q = q.bind(st);
    }
    if let Some(l) = limit {
        q = q.bind(l);
    }

    let rows = q.fetch_all(&db.pool).await?;
    Ok(rows)
}

pub async fn create_reminder(
    db: &Db,
    id: &str,
    input: ReminderInput,
) -> Result<Reminder, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "INSERT INTO reminders (
            id, created_at, updated_at, task_id, due_date, snoozed_until
         ) VALUES ($1, $2, $3, $4, $5, $6)",
        db,
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .bind(&input.task_id)
    .bind(&input.due_date)
    .bind(&input.snoozed_until)
    .execute(&db.pool)
    .await?;
    get_reminder(db, id).await
}

pub async fn get_reminder(db: &Db, id: &str) -> Result<Reminder, AppError> {
    let row = query_as::<Reminder>("SELECT * FROM reminders WHERE id = $1", db)
        .bind(id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn get_reminder_by_task(db: &Db, task_id: &str) -> Result<Reminder, AppError> {
    let row = query_as::<Reminder>("SELECT * FROM reminders WHERE task_id = $1", db)
        .bind(task_id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn update_reminder(
    db: &Db,
    id: &str,
    input: ReminderInput,
) -> Result<Reminder, AppError> {
    let _ = get_reminder(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "UPDATE reminders
         SET updated_at = $1, due_date = $2, snoozed_until = $3
         WHERE id = $4",
        db,
    )
    .bind(&now)
    .bind(&input.due_date)
    .bind(&input.snoozed_until)
    .bind(id)
    .execute(&db.pool)
    .await?;
    get_reminder(db, id).await
}

pub async fn upsert_reminder(
    db: &Db,
    task_id: &str,
    due_date: &str,
    snoozed_until: Option<&str>,
) -> Result<Reminder, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    let existing = query_as::<Reminder>("SELECT * FROM reminders WHERE task_id = $1", db)
        .bind(task_id)
        .fetch_optional(&db.pool)
        .await?;

    if let Some(existing) = existing {
        query(
            "UPDATE reminders
             SET updated_at = $1, due_date = $2, snoozed_until = $3
             WHERE id = $4",
            db,
        )
        .bind(&now)
        .bind(due_date)
        .bind(snoozed_until)
        .bind(&existing.id)
        .execute(&db.pool)
        .await?;
        get_reminder(db, &existing.id).await
    } else {
        let id = uuid::Uuid::now_v7().to_string();
        query(
            "INSERT INTO reminders (
                id, created_at, updated_at, task_id, due_date, snoozed_until
             ) VALUES ($1, $2, $3, $4, $5, $6)",
            db,
        )
        .bind(&id)
        .bind(&now)
        .bind(&now)
        .bind(task_id)
        .bind(due_date)
        .bind(snoozed_until)
        .execute(&db.pool)
        .await?;
        get_reminder(db, &id).await
    }
}

pub async fn snooze_reminder(
    db: &Db,
    task_id: &str,
    until_date: &str,
) -> Result<Reminder, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = query(
        "UPDATE reminders SET updated_at = $1, snoozed_until = $2 WHERE task_id = $3",
        db,
    )
    .bind(&now)
    .bind(until_date)
    .bind(task_id)
    .execute(&db.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    get_reminder_by_task(db, task_id).await
}

pub async fn delete_reminder(db: &Db, id: &str) -> Result<(), AppError> {
    let result = query("DELETE FROM reminders WHERE id = $1", db)
        .bind(id)
        .execute(&db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn complete_task_transaction(
    db: &Db,
    log_entry_id: &str,
    task_id: &str,
    asset_id: Option<&str>,
    kind: &str,
    completed_date: NaiveDate,
    cost_cents: Option<i64>,
    vendor: Option<&str>,
    performed_by: Option<&str>,
    notes: Option<&str>,
) -> Result<crate::repo::log::LogEntry, AppError> {
    use crate::repo::log::LogEntry;

    let task = query_as::<TaskForReminder>("SELECT * FROM tasks WHERE id = $1 AND active = 1", db)
        .bind(task_id)
        .fetch_optional(&db.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut tx = db.pool.begin().await?;
    let now = chrono::Utc::now().to_rfc3339();
    let completed_iso = completed_date.to_string();

    let _ = query(
        "INSERT INTO log_entries (
            id, created_at, updated_at, task_id, asset_id, kind,
            scheduled_date, completed_date, cost_cents, vendor, performed_by, notes
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        db,
    )
    .bind(log_entry_id)
    .bind(&now)
    .bind(&now)
    .bind(task_id)
    .bind(asset_id)
    .bind(kind)
    .bind(None::<&str>)
    .bind(&completed_iso)
    .bind(cost_cents)
    .bind(vendor)
    .bind(performed_by)
    .bind(notes)
    .execute(&mut *tx)
    .await?;

    query("UPDATE tasks SET updated_at = $1 WHERE id = $2", db)
        .bind(&now)
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

    let next_due = compute_next_due(&task, completed_date)?;
    let next_due_str = next_due.to_string();

    let existing = query_as::<Reminder>("SELECT * FROM reminders WHERE task_id = $1", db)
        .bind(task_id)
        .fetch_optional(&mut *tx)
        .await?;

    if let Some(existing) = existing {
        query(
            "UPDATE reminders
             SET updated_at = $1, due_date = $2, snoozed_until = NULL
             WHERE id = $3",
            db,
        )
        .bind(&now)
        .bind(&next_due_str)
        .bind(&existing.id)
        .execute(&mut *tx)
        .await?;
    } else {
        let reminder_id = uuid::Uuid::now_v7().to_string();
        query(
            "INSERT INTO reminders (
                id, created_at, updated_at, task_id, due_date, snoozed_until
             ) VALUES ($1, $2, $3, $4, $5, $6)",
            db,
        )
        .bind(&reminder_id)
        .bind(&now)
        .bind(&now)
        .bind(task_id)
        .bind(&next_due_str)
        .bind(None::<&str>)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let log_entry = query_as::<LogEntry>("SELECT * FROM log_entries WHERE id = $1", db)
        .bind(log_entry_id)
        .fetch_optional(&db.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(log_entry)
}

#[derive(Debug, FromRow)]
struct TaskForReminder {
    schedule_mode: String,
    interval_value: Option<i64>,
    interval_unit: Option<String>,
    season_anchor: Option<String>,
    fixed_interval_years: Option<i64>,
}

fn compute_next_due(
    task: &TaskForReminder,
    completed_date: NaiveDate,
) -> Result<NaiveDate, AppError> {
    match task.schedule_mode.as_str() {
        "floating" => {
            let value = task.interval_value.unwrap_or(1) as u32;
            let unit = task.interval_unit.as_deref().unwrap_or("month");
            next_due_floating(completed_date, value, unit)
                .map_err(|e| AppError::Internal(e.to_string()))
        }
        "fixed" => {
            let anchor = task
                .season_anchor
                .as_deref()
                .ok_or_else(|| AppError::Internal("missing season_anchor".to_string()))?;
            let years = task.fixed_interval_years.unwrap_or(1) as u32;
            next_due_fixed(completed_date, anchor, years)
                .map_err(|e| AppError::Internal(e.to_string()))
        }
        _ => Err(AppError::Internal(format!(
            "unknown schedule mode: {}",
            task.schedule_mode
        ))),
    }
}

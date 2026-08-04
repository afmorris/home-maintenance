#![allow(dead_code)]
use crate::db::{Db, query, query_as};
use crate::domain::recurrence::initial_due_date;
use crate::error::AppError;
use crate::repo::reminders::upsert_reminder;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct Task {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub asset_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub schedule_mode: String,
    pub interval_value: Option<i64>,
    pub interval_unit: Option<String>,
    pub season_anchor: Option<String>,
    pub fixed_interval_years: Option<i64>,
    pub estimated_minutes: Option<i64>,
    pub active: i64,
}

#[derive(Debug, Default)]
pub struct TaskInput {
    pub asset_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub schedule_mode: String,
    pub interval_value: Option<i64>,
    pub interval_unit: Option<String>,
    pub season_anchor: Option<String>,
    pub fixed_interval_years: Option<i64>,
    pub estimated_minutes: Option<i64>,
}

pub async fn list_tasks(db: &Db) -> Result<Vec<Task>, AppError> {
    let rows = query_as::<Task>("SELECT * FROM tasks WHERE active = 1 ORDER BY name", db)
        .fetch_all(&db.pool)
        .await?;
    Ok(rows)
}

pub async fn create_task(
    db: &Db,
    id: &str,
    input: TaskInput,
    today: NaiveDate,
    last_done: Option<NaiveDate>,
) -> Result<Task, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "INSERT INTO tasks (
            id, created_at, updated_at, asset_id, name, description,
            schedule_mode, interval_value, interval_unit, season_anchor,
            fixed_interval_years, estimated_minutes, active
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        db,
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .bind(&input.asset_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.schedule_mode)
    .bind(input.interval_value)
    .bind(&input.interval_unit)
    .bind(&input.season_anchor)
    .bind(input.fixed_interval_years)
    .bind(input.estimated_minutes)
    .bind(1)
    .execute(&db.pool)
    .await?;

    let due_date = compute_initial_due_date(&input, today, last_done)?;
    upsert_reminder(db, id, &due_date.to_string(), None).await?;

    get_task(db, id).await
}

fn compute_initial_due_date(
    input: &TaskInput,
    today: NaiveDate,
    last_done: Option<NaiveDate>,
) -> Result<NaiveDate, AppError> {
    let value = input.interval_value.map(|v| v as u32);
    let unit = input.interval_unit.as_deref();
    let anchor = input.season_anchor.as_deref();
    let years = input.fixed_interval_years.unwrap_or(1) as u32;

    initial_due_date(
        today,
        last_done,
        &input.schedule_mode,
        value,
        unit,
        anchor,
        years,
    )
    .map_err(|e| AppError::BadRequest(e.to_string()))
}

pub async fn recompute_reminder_for_task(
    db: &Db,
    task_id: &str,
    today: NaiveDate,
    last_done: Option<NaiveDate>,
) -> Result<(), AppError> {
    let task = get_task(db, task_id).await?;
    let input = TaskInput {
        asset_id: task.asset_id,
        name: task.name,
        description: task.description,
        schedule_mode: task.schedule_mode,
        interval_value: task.interval_value,
        interval_unit: task.interval_unit,
        season_anchor: task.season_anchor,
        fixed_interval_years: task.fixed_interval_years,
        estimated_minutes: task.estimated_minutes,
    };
    let due_date = compute_initial_due_date(&input, today, last_done)?;
    upsert_reminder(db, task_id, &due_date.to_string(), None).await?;
    Ok(())
}

pub async fn get_task(db: &Db, id: &str) -> Result<Task, AppError> {
    let row = query_as::<Task>("SELECT * FROM tasks WHERE id = $1 AND active = 1", db)
        .bind(id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn update_task(
    db: &Db,
    id: &str,
    input: TaskInput,
    today: NaiveDate,
    last_done: Option<NaiveDate>,
) -> Result<Task, AppError> {
    let existing = get_task(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();

    let name = if input.name.is_empty() {
        existing.name.clone()
    } else {
        input.name
    };
    let asset_id = input.asset_id.or(existing.asset_id.clone());
    let description = input.description.or(existing.description.clone());
    let schedule_mode = if input.schedule_mode.is_empty() {
        existing.schedule_mode.clone()
    } else {
        input.schedule_mode
    };
    let interval_value = input.interval_value.or(existing.interval_value);
    let interval_unit = input.interval_unit.or(existing.interval_unit.clone());
    let season_anchor = input.season_anchor.or(existing.season_anchor.clone());
    let fixed_interval_years = input.fixed_interval_years.or(existing.fixed_interval_years);
    let estimated_minutes = input.estimated_minutes.or(existing.estimated_minutes);

    let schedule_changed = existing.schedule_mode != schedule_mode
        || existing.interval_value != interval_value
        || existing.interval_unit != interval_unit
        || existing.season_anchor != season_anchor
        || existing.fixed_interval_years != fixed_interval_years;

    query(
        "UPDATE tasks SET
            updated_at = $1, asset_id = $2, name = $3, description = $4,
            schedule_mode = $5, interval_value = $6, interval_unit = $7,
            season_anchor = $8, fixed_interval_years = $9, estimated_minutes = $10
         WHERE id = $11",
        db,
    )
    .bind(&now)
    .bind(&asset_id)
    .bind(&name)
    .bind(&description)
    .bind(&schedule_mode)
    .bind(interval_value)
    .bind(&interval_unit)
    .bind(&season_anchor)
    .bind(fixed_interval_years)
    .bind(estimated_minutes)
    .bind(id)
    .execute(&db.pool)
    .await?;

    if schedule_changed {
        let merged_input = TaskInput {
            asset_id: asset_id.clone(),
            name: name.clone(),
            description: description.clone(),
            schedule_mode: schedule_mode.clone(),
            interval_value,
            interval_unit: interval_unit.clone(),
            season_anchor: season_anchor.clone(),
            fixed_interval_years,
            estimated_minutes,
        };
        let due_date = compute_initial_due_date(&merged_input, today, last_done)?;
        upsert_reminder(db, id, &due_date.to_string(), None).await?;
    }

    get_task(db, id).await
}

pub async fn delete_task(db: &Db, id: &str) -> Result<(), AppError> {
    let now = chrono::Utc::now().to_rfc3339();

    let mut tx = db.pool.begin().await?;

    let result = query(
        "UPDATE tasks SET active = 0, updated_at = $1 WHERE id = $2 AND active = 1",
        db,
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    query("DELETE FROM reminders WHERE task_id = $1", db)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

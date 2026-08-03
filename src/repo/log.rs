#![allow(dead_code)]
use crate::db::{Db, query, query_as};
use crate::error::AppError;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct LogEntry {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub task_id: Option<String>,
    pub asset_id: Option<String>,
    pub kind: String,
    pub scheduled_date: Option<String>,
    pub completed_date: String,
    pub cost_cents: Option<i64>,
    pub vendor: Option<String>,
    pub performed_by: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default)]
pub struct LogEntryInput {
    pub task_id: Option<String>,
    pub asset_id: Option<String>,
    pub kind: String,
    pub scheduled_date: Option<String>,
    pub completed_date: String,
    pub cost_cents: Option<i64>,
    pub vendor: Option<String>,
    pub performed_by: Option<String>,
    pub notes: Option<String>,
}

pub async fn list_log_entries(
    db: &Db,
    asset_id: Option<&str>,
    kind: Option<&str>,
    tag: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<LogEntry>, AppError> {
    let mut sql = String::from(
        "SELECT DISTINCT le.* FROM log_entries le
         LEFT JOIN entry_tags et ON et.entry_id = le.id
         LEFT JOIN tags tg ON tg.id = et.tag_id
         WHERE 1 = 1",
    );

    if asset_id.is_some() {
        sql.push_str(" AND le.asset_id = $P1");
    }
    if kind.is_some() {
        sql.push_str(" AND le.kind = $P2");
    }
    if tag.is_some() {
        sql.push_str(" AND tg.name = $P3");
    }
    if from.is_some() {
        sql.push_str(" AND le.completed_date >= $P4");
    }
    if to.is_some() {
        sql.push_str(" AND le.completed_date <= $P5");
    }
    if search.is_some() {
        sql.push_str(" AND (le.notes LIKE $P6 OR le.vendor LIKE $P6 OR le.performed_by LIKE $P6)");
    }

    sql.push_str(" ORDER BY le.completed_date DESC");

    let mut next = 1usize;
    let mut p_sql = sql;
    for token in ["$P1", "$P2", "$P3", "$P4", "$P5", "$P6"] {
        if p_sql.contains(token) {
            p_sql = p_sql.replace(token, &format!("${}", next));
            next += 1;
        }
    }

    let static_sql: &'static str = Box::leak(p_sql.into_boxed_str());
    let mut q = query_as::<LogEntry>(static_sql, db);

    if let Some(v) = asset_id {
        q = q.bind(v);
    }
    if let Some(v) = kind {
        q = q.bind(v);
    }
    if let Some(v) = tag {
        q = q.bind(v);
    }
    if let Some(v) = from {
        q = q.bind(v);
    }
    if let Some(v) = to {
        q = q.bind(v);
    }
    if let Some(v) = search {
        let pattern = format!("%{}%", v);
        q = q.bind(pattern);
    }

    let rows = q.fetch_all(&db.pool).await?;
    Ok(rows)
}

pub async fn create_log_entry(
    db: &Db,
    id: &str,
    input: LogEntryInput,
) -> Result<LogEntry, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "INSERT INTO log_entries (
            id, created_at, updated_at, task_id, asset_id, kind,
            scheduled_date, completed_date, cost_cents, vendor, performed_by, notes
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        db,
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .bind(&input.task_id)
    .bind(&input.asset_id)
    .bind(&input.kind)
    .bind(&input.scheduled_date)
    .bind(&input.completed_date)
    .bind(input.cost_cents)
    .bind(&input.vendor)
    .bind(&input.performed_by)
    .bind(&input.notes)
    .execute(&db.pool)
    .await?;
    get_log_entry(db, id).await
}

pub async fn get_log_entry(db: &Db, id: &str) -> Result<LogEntry, AppError> {
    let row = query_as::<LogEntry>("SELECT * FROM log_entries WHERE id = $1", db)
        .bind(id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn update_log_entry(
    db: &Db,
    id: &str,
    input: LogEntryInput,
) -> Result<LogEntry, AppError> {
    let _ = get_log_entry(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "UPDATE log_entries SET
            updated_at = $1, task_id = $2, asset_id = $3, kind = $4,
            scheduled_date = $5, completed_date = $6, cost_cents = $7,
            vendor = $8, performed_by = $9, notes = $10
         WHERE id = $11",
        db,
    )
    .bind(&now)
    .bind(&input.task_id)
    .bind(&input.asset_id)
    .bind(&input.kind)
    .bind(&input.scheduled_date)
    .bind(&input.completed_date)
    .bind(input.cost_cents)
    .bind(&input.vendor)
    .bind(&input.performed_by)
    .bind(&input.notes)
    .bind(id)
    .execute(&db.pool)
    .await?;
    get_log_entry(db, id).await
}

pub async fn delete_log_entry(db: &Db, id: &str, confirm: bool) -> Result<(), AppError> {
    if !confirm {
        return Err(AppError::BadRequest("confirm=true required".to_string()));
    }
    let result = query("DELETE FROM log_entries WHERE id = $1", db)
        .bind(id)
        .execute(&db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

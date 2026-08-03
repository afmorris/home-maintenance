#![allow(dead_code)]
use crate::db::{Db, query, query_as};
use crate::error::AppError;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct Location {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub name: String,
    pub kind: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct LocationInput {
    pub name: String,
    pub kind: String,
    pub parent_id: Option<String>,
}

pub async fn list_locations(db: &Db) -> Result<Vec<Location>, AppError> {
    let rows = query_as::<Location>("SELECT * FROM locations ORDER BY name", db)
        .fetch_all(&db.pool)
        .await?;
    Ok(rows)
}

pub async fn create_location(
    db: &Db,
    id: &str,
    input: LocationInput,
) -> Result<Location, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "INSERT INTO locations (id, created_at, updated_at, name, kind, parent_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
        db,
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .bind(&input.name)
    .bind(&input.kind)
    .bind(&input.parent_id)
    .execute(&db.pool)
    .await?;
    get_location(db, id).await
}

pub async fn get_location(db: &Db, id: &str) -> Result<Location, AppError> {
    let row = query_as::<Location>("SELECT * FROM locations WHERE id = $1", db)
        .bind(id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn update_location(
    db: &Db,
    id: &str,
    input: LocationInput,
) -> Result<Location, AppError> {
    let _ = get_location(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "UPDATE locations
         SET updated_at = $1, name = $2, kind = $3, parent_id = $4
         WHERE id = $5",
        db,
    )
    .bind(&now)
    .bind(&input.name)
    .bind(&input.kind)
    .bind(&input.parent_id)
    .bind(id)
    .execute(&db.pool)
    .await?;
    get_location(db, id).await
}

pub async fn delete_location(db: &Db, id: &str) -> Result<(), AppError> {
    let result = query("DELETE FROM locations WHERE id = $1", db)
        .bind(id)
        .execute(&db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

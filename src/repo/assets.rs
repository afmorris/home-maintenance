#![allow(dead_code)]
use crate::db::{Db, query, query_as};
use crate::error::AppError;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct Asset {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub name: String,
    pub location_id: Option<String>,
    pub category: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub install_date: Option<String>,
    pub warranty_end: Option<String>,
    pub notes: Option<String>,
    pub archived: i64,
}

#[derive(Debug, Default)]
pub struct AssetInput {
    pub name: String,
    pub location_id: Option<String>,
    pub category: String,
    pub make: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub install_date: Option<String>,
    pub warranty_end: Option<String>,
    pub notes: Option<String>,
}

pub async fn list_assets(db: &Db) -> Result<Vec<Asset>, AppError> {
    let rows = query_as::<Asset>("SELECT * FROM assets WHERE archived = 0 ORDER BY name", db)
        .fetch_all(&db.pool)
        .await?;
    Ok(rows)
}

pub async fn create_asset(db: &Db, id: &str, input: AssetInput) -> Result<Asset, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "INSERT INTO assets (
            id, created_at, updated_at, name, location_id, category,
            make, model, serial, install_date, warranty_end, notes, archived
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        db,
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .bind(&input.name)
    .bind(&input.location_id)
    .bind(&input.category)
    .bind(&input.make)
    .bind(&input.model)
    .bind(&input.serial)
    .bind(&input.install_date)
    .bind(&input.warranty_end)
    .bind(&input.notes)
    .bind(0)
    .execute(&db.pool)
    .await?;
    get_asset(db, id).await
}

pub async fn get_asset(db: &Db, id: &str) -> Result<Asset, AppError> {
    let row = query_as::<Asset>("SELECT * FROM assets WHERE id = $1", db)
        .bind(id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn update_asset(db: &Db, id: &str, input: AssetInput) -> Result<Asset, AppError> {
    let existing = get_asset(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();

    let name = if input.name.is_empty() {
        existing.name
    } else {
        input.name
    };
    let location_id = input.location_id.or(existing.location_id);
    let category = if input.category.is_empty() {
        existing.category
    } else {
        input.category
    };
    let make = merge_opt(input.make, existing.make);
    let model = merge_opt(input.model, existing.model);
    let serial = merge_opt(input.serial, existing.serial);
    let install_date = merge_opt(input.install_date, existing.install_date);
    let warranty_end = merge_opt(input.warranty_end, existing.warranty_end);
    let notes = merge_opt(input.notes, existing.notes);

    query(
        "UPDATE assets SET
            updated_at = $1, name = $2, location_id = $3, category = $4,
            make = $5, model = $6, serial = $7, install_date = $8,
            warranty_end = $9, notes = $10
         WHERE id = $11",
        db,
    )
    .bind(&now)
    .bind(&name)
    .bind(&location_id)
    .bind(&category)
    .bind(&make)
    .bind(&model)
    .bind(&serial)
    .bind(&install_date)
    .bind(&warranty_end)
    .bind(&notes)
    .bind(id)
    .execute(&db.pool)
    .await?;
    get_asset(db, id).await
}

pub async fn archive_asset(db: &Db, id: &str) -> Result<Asset, AppError> {
    let _ = get_asset(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "UPDATE assets SET updated_at = $1, archived = 1 WHERE id = $2",
        db,
    )
    .bind(&now)
    .bind(id)
    .execute(&db.pool)
    .await?;
    get_asset(db, id).await
}

pub async fn delete_asset(db: &Db, id: &str) -> Result<(), AppError> {
    let result = query("DELETE FROM assets WHERE id = $1", db)
        .bind(id)
        .execute(&db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

fn merge_opt(input: Option<String>, existing: Option<String>) -> Option<String> {
    match input {
        Some(ref s) if s.is_empty() => existing,
        Some(s) => Some(s),
        None => existing,
    }
}

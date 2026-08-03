#![allow(dead_code)]
use crate::db::{Db, query, query_as};
use crate::error::AppError;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct Supply {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub name: String,
    pub spec: Option<String>,
    pub purchase_url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default)]
pub struct SupplyInput {
    pub name: String,
    pub spec: Option<String>,
    pub purchase_url: Option<String>,
    pub notes: Option<String>,
}

pub async fn list_supplies(db: &Db) -> Result<Vec<Supply>, AppError> {
    let rows = query_as::<Supply>("SELECT * FROM supplies ORDER BY name", db)
        .fetch_all(&db.pool)
        .await?;
    Ok(rows)
}

pub async fn create_supply(db: &Db, id: &str, input: SupplyInput) -> Result<Supply, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "INSERT INTO supplies (
            id, created_at, updated_at, name, spec, purchase_url, notes
         ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        db,
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .bind(&input.name)
    .bind(&input.spec)
    .bind(&input.purchase_url)
    .bind(&input.notes)
    .execute(&db.pool)
    .await?;
    get_supply(db, id).await
}

pub async fn get_supply(db: &Db, id: &str) -> Result<Supply, AppError> {
    let row = query_as::<Supply>("SELECT * FROM supplies WHERE id = $1", db)
        .bind(id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn update_supply(db: &Db, id: &str, input: SupplyInput) -> Result<Supply, AppError> {
    let _ = get_supply(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "UPDATE supplies
         SET updated_at = $1, name = $2, spec = $3, purchase_url = $4, notes = $5
         WHERE id = $6",
        db,
    )
    .bind(&now)
    .bind(&input.name)
    .bind(&input.spec)
    .bind(&input.purchase_url)
    .bind(&input.notes)
    .bind(id)
    .execute(&db.pool)
    .await?;
    get_supply(db, id).await
}

pub async fn delete_supply(db: &Db, id: &str) -> Result<(), AppError> {
    let result = query("DELETE FROM supplies WHERE id = $1", db)
        .bind(id)
        .execute(&db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

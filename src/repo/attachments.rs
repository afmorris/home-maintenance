#![allow(dead_code)]
use crate::db::{Db, query, query_as};
use crate::error::AppError;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct Attachment {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub owner_type: String,
    pub owner_id: String,
    pub filename: String,
    pub content_type: String,
    pub byte_size: i64,
    pub caption: Option<String>,
}

#[derive(Debug, Default)]
pub struct AttachmentInput {
    pub owner_type: String,
    pub owner_id: String,
    pub filename: String,
    pub content_type: String,
    pub byte_size: i64,
    pub caption: Option<String>,
}

pub async fn list_attachments_for_owner(
    db: &Db,
    owner_type: &str,
    owner_id: &str,
) -> Result<Vec<Attachment>, AppError> {
    let rows = query_as::<Attachment>(
        "SELECT * FROM attachments WHERE owner_type = $1 AND owner_id = $2 ORDER BY created_at",
        db,
    )
    .bind(owner_type)
    .bind(owner_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn create_attachment(
    db: &Db,
    id: &str,
    input: AttachmentInput,
) -> Result<Attachment, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "INSERT INTO attachments (id, created_at, updated_at, owner_type, owner_id, filename, content_type, byte_size, caption)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        db,
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .bind(&input.owner_type)
    .bind(&input.owner_id)
    .bind(&input.filename)
    .bind(&input.content_type)
    .bind(input.byte_size)
    .bind(&input.caption)
    .execute(&db.pool)
    .await?;
    get_attachment(db, id).await
}

pub async fn get_attachment(db: &Db, id: &str) -> Result<Attachment, AppError> {
    let row = query_as::<Attachment>("SELECT * FROM attachments WHERE id = $1", db)
        .bind(id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn update_attachment(
    db: &Db,
    id: &str,
    caption: Option<String>,
) -> Result<Attachment, AppError> {
    let _ = get_attachment(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "UPDATE attachments SET updated_at = $1, caption = $2 WHERE id = $3",
        db,
    )
    .bind(&now)
    .bind(&caption)
    .bind(id)
    .execute(&db.pool)
    .await?;
    get_attachment(db, id).await
}

pub async fn delete_attachment(db: &Db, id: &str) -> Result<(), AppError> {
    let result = query("DELETE FROM attachments WHERE id = $1", db)
        .bind(id)
        .execute(&db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

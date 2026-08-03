#![allow(dead_code)]
use crate::db::{Db, query, query_as};
use crate::error::AppError;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct Tag {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub name: String,
}

#[derive(Debug, Default)]
pub struct TagInput {
    pub name: String,
}

#[derive(Debug, FromRow, Serialize)]
pub struct TagForEntry {
    pub id: String,
    pub name: String,
}

pub async fn list_tags(db: &Db) -> Result<Vec<Tag>, AppError> {
    let rows = query_as::<Tag>("SELECT * FROM tags ORDER BY name", db)
        .fetch_all(&db.pool)
        .await?;
    Ok(rows)
}

pub async fn create_tag(db: &Db, id: &str, input: TagInput) -> Result<Tag, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "INSERT INTO tags (id, created_at, updated_at, name) VALUES ($1, $2, $3, $4)",
        db,
    )
    .bind(id)
    .bind(&now)
    .bind(&now)
    .bind(&input.name)
    .execute(&db.pool)
    .await?;
    get_tag(db, id).await
}

pub async fn get_tag(db: &Db, id: &str) -> Result<Tag, AppError> {
    let row = query_as::<Tag>("SELECT * FROM tags WHERE id = $1", db)
        .bind(id)
        .fetch_optional(&db.pool)
        .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn update_tag(db: &Db, id: &str, input: TagInput) -> Result<Tag, AppError> {
    let _ = get_tag(db, id).await?;
    let now = chrono::Utc::now().to_rfc3339();
    query(
        "UPDATE tags SET updated_at = $1, name = $2 WHERE id = $3",
        db,
    )
    .bind(&now)
    .bind(&input.name)
    .bind(id)
    .execute(&db.pool)
    .await?;
    get_tag(db, id).await
}

pub async fn delete_tag(db: &Db, id: &str) -> Result<(), AppError> {
    let result = query("DELETE FROM tags WHERE id = $1", db)
        .bind(id)
        .execute(&db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn attach_tag(db: &Db, entry_id: &str, tag_id: &str) -> Result<(), AppError> {
    query(
        "INSERT INTO entry_tags (entry_id, tag_id) VALUES ($1, $2)",
        db,
    )
    .bind(entry_id)
    .bind(tag_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn detach_tag(db: &Db, entry_id: &str, tag_id: &str) -> Result<(), AppError> {
    let result = query(
        "DELETE FROM entry_tags WHERE entry_id = $1 AND tag_id = $2",
        db,
    )
    .bind(entry_id)
    .bind(tag_id)
    .execute(&db.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn list_tags_for_entry(db: &Db, entry_id: &str) -> Result<Vec<TagForEntry>, AppError> {
    let rows = query_as::<TagForEntry>(
        "SELECT t.id, t.name FROM tags t
         JOIN entry_tags et ON et.tag_id = t.id
         WHERE et.entry_id = $1
         ORDER BY t.name",
        db,
    )
    .bind(entry_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

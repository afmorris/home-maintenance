use crate::config::CONFIG;
use crate::error::AppError;
use crate::repo;
use crate::repo::attachments::AttachmentInput;
use crate::web::AppState;
use axum::Json;
use axum::extract::{Multipart, State};
use serde_json::json;

pub async fn create_attachment(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut owner_type: Option<String> = None;
    let mut owner_id: Option<String> = None;
    let mut file_info: Option<FileInfo> = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "owner_type" => {
                owner_type = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::BadRequest(e.to_string()))?,
                );
            }
            "owner_id" => {
                owner_id = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::BadRequest(e.to_string()))?,
                );
            }
            "file" => {
                let original_filename = field.file_name().unwrap_or("upload").to_string();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();

                let id = uuid::Uuid::now_v7().to_string();
                let safe_filename = sanitize_filename(&original_filename);
                let stored_filename = format!("{}_{}", id, safe_filename);

                let dir = CONFIG
                    .data_dir
                    .join("attachments")
                    .join(owner_type.as_deref().unwrap_or("unknown"))
                    .join(owner_id.as_deref().unwrap_or("unknown"));
                std::fs::create_dir_all(&dir).map_err(|e| AppError::Internal(e.to_string()))?;

                let path = dir.join(&stored_filename);
                let mut file = tokio::fs::File::create(&path)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;

                let mut byte_size: i64 = 0;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?
                {
                    byte_size += chunk.len() as i64;
                    tokio::io::copy(&mut chunk.as_ref(), &mut file)
                        .await
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                }

                file_info = Some(FileInfo {
                    id,
                    owner_type: owner_type.clone().unwrap_or_default(),
                    owner_id: owner_id.clone().unwrap_or_default(),
                    original_filename,
                    content_type,
                    byte_size,
                    path,
                });
            }
            _ => {}
        }
    }

    let owner_type =
        owner_type.ok_or_else(|| AppError::BadRequest("owner_type required".to_string()))?;
    let owner_id = owner_id.ok_or_else(|| AppError::BadRequest("owner_id required".to_string()))?;
    let info = file_info.ok_or_else(|| AppError::BadRequest("file required".to_string()))?;

    let attachment = repo::attachments::create_attachment(
        &state.db,
        &info.id,
        AttachmentInput {
            owner_type,
            owner_id,
            filename: info.original_filename,
            content_type: info.content_type,
            byte_size: info.byte_size,
            caption: None,
        },
    )
    .await?;

    let attachment_id = attachment.id.clone();

    Ok(Json(json!({
        "id": attachment_id,
        "attachment": attachment,
        "path": info.path.to_string_lossy(),
    })))
}

struct FileInfo {
    id: String,
    #[allow(dead_code)]
    owner_type: String,
    #[allow(dead_code)]
    owner_id: String,
    original_filename: String,
    content_type: String,
    byte_size: i64,
    path: std::path::PathBuf,
}

fn sanitize_filename(name: &str) -> String {
    let name = std::path::Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload");
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

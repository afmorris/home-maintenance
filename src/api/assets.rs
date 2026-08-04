use crate::error::AppError;
use crate::repo::assets::{self, AssetInput};
use crate::web::AppState;
use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use serde_json::json;

pub async fn list_assets(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = assets::list_assets(&state.db).await?;
    Ok(Json(json!({"assets": rows})))
}

#[derive(Deserialize)]
pub struct CreateAssetPayload {
    name: String,
    location_id: Option<String>,
    category: String,
    make: Option<String>,
    model: Option<String>,
    serial: Option<String>,
    install_date: Option<String>,
    warranty_end: Option<String>,
    notes: Option<String>,
}

pub async fn create_asset(
    State(state): State<AppState>,
    Json(payload): Json<CreateAssetPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::now_v7().to_string();
    let input = AssetInput {
        name: payload.name,
        location_id: payload.location_id,
        category: payload.category,
        make: payload.make,
        model: payload.model,
        serial: payload.serial,
        install_date: payload.install_date,
        warranty_end: payload.warranty_end,
        notes: payload.notes,
    };
    let asset = assets::create_asset(&state.db, &id, input).await?;
    Ok(Json(json!({"id": asset.id, "asset": asset})))
}

pub async fn get_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let asset = assets::get_asset(&state.db, &id).await?;
    Ok(Json(json!({"asset": asset})))
}

#[derive(Deserialize)]
pub struct UpdateAssetPayload {
    name: Option<String>,
    location_id: Option<String>,
    category: Option<String>,
    make: Option<String>,
    model: Option<String>,
    serial: Option<String>,
    install_date: Option<String>,
    warranty_end: Option<String>,
    notes: Option<String>,
}

pub async fn update_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAssetPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let input = AssetInput {
        name: payload.name.unwrap_or_default(),
        location_id: payload.location_id,
        category: payload.category.unwrap_or_default(),
        make: payload.make,
        model: payload.model,
        serial: payload.serial,
        install_date: payload.install_date,
        warranty_end: payload.warranty_end,
        notes: payload.notes,
    };
    let asset = assets::update_asset(&state.db, &id, input).await?;
    Ok(Json(json!({"id": asset.id, "asset": asset})))
}

pub async fn delete_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    assets::delete_asset(&state.db, &id).await?;
    Ok(Json(json!({"deleted": true})))
}

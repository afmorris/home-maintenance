use crate::error::AppError;
use crate::repo::supplies::{self, SupplyInput};
use crate::web::AppState;
use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use serde_json::json;

pub async fn list_supplies(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = supplies::list_supplies(&state.db).await?;
    Ok(Json(json!({"supplies": rows})))
}

#[derive(Deserialize)]
pub struct CreateSupplyPayload {
    name: String,
    spec: Option<String>,
    purchase_url: Option<String>,
    notes: Option<String>,
}

pub async fn create_supply(
    State(state): State<AppState>,
    Json(payload): Json<CreateSupplyPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::now_v7().to_string();
    let input = SupplyInput {
        name: payload.name,
        spec: payload.spec,
        purchase_url: payload.purchase_url,
        notes: payload.notes,
    };
    let supply = supplies::create_supply(&state.db, &id, input).await?;
    Ok(Json(json!({"id": supply.id, "supply": supply})))
}

pub async fn get_supply(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let supply = supplies::get_supply(&state.db, &id).await?;
    Ok(Json(json!({"supply": supply})))
}

#[derive(Deserialize)]
pub struct UpdateSupplyPayload {
    name: Option<String>,
    spec: Option<String>,
    purchase_url: Option<String>,
    notes: Option<String>,
}

pub async fn update_supply(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateSupplyPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let input = SupplyInput {
        name: payload.name.unwrap_or_default(),
        spec: payload.spec,
        purchase_url: payload.purchase_url,
        notes: payload.notes,
    };
    let supply = supplies::update_supply(&state.db, &id, input).await?;
    Ok(Json(json!({"id": supply.id, "supply": supply})))
}

pub async fn delete_supply(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    supplies::delete_supply(&state.db, &id).await?;
    Ok(Json(json!({"deleted": true})))
}

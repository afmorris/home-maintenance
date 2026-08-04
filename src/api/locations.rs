use crate::error::AppError;
use crate::repo::locations::{self, LocationInput};
use crate::web::AppState;
use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use serde_json::json;

pub async fn list_locations(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows = locations::list_locations(&state.db).await?;
    Ok(Json(json!({"locations": rows})))
}

#[derive(Deserialize)]
pub struct CreateLocationPayload {
    name: String,
    kind: String,
    parent_id: Option<String>,
}

pub async fn create_location(
    State(state): State<AppState>,
    Json(payload): Json<CreateLocationPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::now_v7().to_string();
    let input = LocationInput {
        name: payload.name,
        kind: payload.kind,
        parent_id: payload.parent_id,
    };
    let location = locations::create_location(&state.db, &id, input).await?;
    Ok(Json(json!({"id": location.id, "location": location})))
}

pub async fn get_location(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let location = locations::get_location(&state.db, &id).await?;
    Ok(Json(json!({"location": location})))
}

#[derive(Deserialize)]
pub struct UpdateLocationPayload {
    name: String,
    kind: String,
    parent_id: Option<String>,
}

pub async fn update_location(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateLocationPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let input = LocationInput {
        name: payload.name,
        kind: payload.kind,
        parent_id: payload.parent_id,
    };
    let location = locations::update_location(&state.db, &id, input).await?;
    Ok(Json(json!({"id": location.id, "location": location})))
}

pub async fn delete_location(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    locations::delete_location(&state.db, &id).await?;
    Ok(Json(json!({"deleted": true})))
}

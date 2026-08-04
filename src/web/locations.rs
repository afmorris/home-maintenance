use crate::error::AppError;
use crate::repo::locations::{self, LocationInput};
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::{Path, State};
use axum::response::Redirect;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "locations.html")]
pub struct LocationListView {
    pub title: String,
    pub locations: Vec<locations::Location>,
}

pub async fn list_locations(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let locations = locations::list_locations(&state.db).await?;
    LocationListView {
        title: "Locations".to_string(),
        locations,
    }
    .render_response()
}

#[derive(Deserialize)]
pub struct LocationFormPayload {
    name: String,
    kind: String,
    parent_id: Option<String>,
}

pub async fn create_location(
    State(state): State<AppState>,
    axum::extract::Form(payload): axum::extract::Form<LocationFormPayload>,
) -> Result<Redirect, AppError> {
    let id = uuid::Uuid::now_v7().to_string();
    let input = LocationInput {
        name: payload.name,
        kind: payload.kind,
        parent_id: payload.parent_id,
    };
    locations::create_location(&state.db, &id, input).await?;
    Ok(Redirect::to("/locations"))
}

pub async fn delete_location(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    locations::delete_location(&state.db, &id).await?;
    Ok(Redirect::to("/locations"))
}

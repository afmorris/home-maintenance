use crate::error::AppError;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::State;

#[derive(Template)]
#[template(path = "locations.html")]
pub struct LocationListView {
    pub title: String,
}

pub async fn list_locations(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    LocationListView {
        title: "Locations".to_string(),
    }
    .render_response()
}

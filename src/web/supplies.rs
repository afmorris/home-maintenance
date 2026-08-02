use crate::error::AppError;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::State;

#[derive(Template)]
#[template(path = "supplies.html")]
pub struct SupplyListView {
    pub title: String,
}

pub async fn list_supplies(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    SupplyListView {
        title: "Supplies".to_string(),
    }
    .render_response()
}

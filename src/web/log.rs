use crate::error::AppError;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::State;

#[derive(Template)]
#[template(path = "log.html")]
pub struct LogListView {
    pub title: String,
}

pub async fn list_log(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    LogListView {
        title: "Log".to_string(),
    }
    .render_response()
}

use crate::error::AppError;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::State;

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsView {
    pub title: String,
}

pub async fn settings_page(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    SettingsView {
        title: "Settings".to_string(),
    }
    .render_response()
}

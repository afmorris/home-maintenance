use crate::config::CONFIG;
use crate::error::AppError;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::State;

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardView {
    pub title: String,
}

pub async fn index(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    DashboardView {
        title: format!("Home Maintenance — {}", CONFIG.app_timezone),
    }
    .render_response()
}

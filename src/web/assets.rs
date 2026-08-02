use crate::error::AppError;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::State;

#[derive(Template)]
#[template(path = "assets.html")]
pub struct AssetListView {
    pub title: String,
}

pub async fn list_assets(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    AssetListView {
        title: "Assets".to_string(),
    }
    .render_response()
}

#[derive(Template)]
#[template(path = "asset_form.html")]
pub struct AssetFormView {
    pub title: String,
}

pub async fn new_asset_form(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    AssetFormView {
        title: "New Asset".to_string(),
    }
    .render_response()
}

#[derive(Template)]
#[template(path = "asset_detail.html")]
pub struct AssetDetailView {
    pub title: String,
}

pub async fn view_asset(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    AssetDetailView {
        title: "Asset".to_string(),
    }
    .render_response()
}

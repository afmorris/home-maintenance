use crate::config::{AuthMode, CONFIG};
use crate::db::Db;
use crate::error::AppError;
use askama::Template;
use axum::extract::{FromRef, FromRequestParts, Request, State};
use axum::http::{StatusCode, header, request::Parts};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Router, routing::get};
use cookie::{Cookie, CookieJar, Key};
use std::convert::Infallible;
use std::sync::Arc;
use tracing::info;

pub mod assets;
pub mod dashboard;
pub mod locations;
pub mod log;
pub mod login;
pub mod reminders;
pub mod seed;
pub mod settings;
pub mod supplies;
pub mod tasks;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub session_key: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.session_key.clone()
    }
}

impl FromRef<AppState> for Arc<Db> {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRequestParts<AppState> for AppState {
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(state.clone())
    }
}

pub async fn ui_router(db: Db) -> Router {
    let state = AppState {
        db: Arc::new(db),
        session_key: Key::from(CONFIG.session_secret.as_bytes()),
    };

    let router = Router::new()
        .route("/", get(dashboard::index))
        .route("/assets", get(assets::list_assets))
        .route("/assets/new", get(assets::new_asset_form))
        .route("/assets/{id}", get(assets::view_asset))
        .route("/tasks", get(tasks::list_tasks))
        .route("/tasks/new", get(tasks::new_task_form))
        .route("/tasks/{id}/edit", get(tasks::edit_task_form))
        .route("/log", get(log::list_log))
        .route("/supplies", get(supplies::list_supplies))
        .route("/locations", get(locations::list_locations))
        .route("/settings", get(settings::settings_page))
        .route("/login", get(login::login_page).post(login::login_submit))
        .route("/logout", get(login::logout));

    let auth_router = match CONFIG.auth_mode {
        AuthMode::None => router,
        AuthMode::Password => {
            router.route_layer(from_fn_with_state(state.clone(), auth_middleware))
        }
    };

    auth_router
        .route("/health", get(health))
        .layer(from_fn_with_state(state.clone(), request_log_middleware))
        .with_state(state)
}

async fn request_log_middleware(_state: AppState, req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let resp = next.run(req).await;
    info!("{} {} => {}", method, uri, resp.status().as_u16());
    resp
}

async fn auth_middleware(state: AppState, req: Request, next: Next) -> Response {
    if req.uri().path().starts_with("/login") || req.uri().path().starts_with("/static/") {
        return next.run(req).await;
    }
    let cookie_header = req
        .headers()
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let mut jar = CookieJar::new();
    if let Some(s) = cookie_header {
        for c in s.split(';') {
            let c = c.trim();
            if let Ok(cookie) = Cookie::parse_encoded(c.to_string()) {
                jar.add_original(cookie);
            }
        }
    }
    if let Some(decoded) = jar.signed(&state.session_key).get("session")
        && decoded.value() == "authenticated"
    {
        return next.run(req).await;
    }
    Redirect::to("/login").into_response()
}

async fn health(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let _row: (i64,) = sqlx::query_as("SELECT 1").fetch_one(&state.db.pool).await?;
    Ok((
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "status": "ok",
            "db_backend": state.db.backend.kind(),
            "version": crate::config::VERSION,
        })),
    ))
}

pub async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html("<h1>Not Found</h1><p><a href=\"/\">Go home</a></p>"),
    )
}

pub trait RenderTemplate: Template + Sized {
    fn render_response(self) -> Result<Html<String>, AppError>;
}

impl<T: Template> RenderTemplate for T {
    fn render_response(self) -> Result<Html<String>, AppError> {
        let html = self
            .render()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(Html(html))
    }
}

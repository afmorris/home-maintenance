use crate::config::CONFIG;
use crate::db::Db;
use crate::error::AppError;
use crate::web::AppState;
use axum::Router;
use axum::extract::Request;
use axum::http::header;
use axum::middleware::{Next, from_fn};
use axum::response::Response;
use axum::routing::{get, post};

pub mod assets;
pub mod health;
pub mod log;
pub mod reminders;
pub mod stats;
pub mod supplies;
pub mod tasks;

pub fn api_router(db: Db) -> Router {
    let state = AppState {
        db: std::sync::Arc::new(db),
        session_key: cookie::Key::from(CONFIG.session_secret.as_bytes()),
    };

    Router::new()
        .route("/api/v1/health", get(health::health))
        .route("/api/v1/reminders", get(reminders::list_reminders))
        .route(
            "/api/v1/tasks",
            get(tasks::list_tasks).post(tasks::create_task),
        )
        .route(
            "/api/v1/tasks/{id}",
            get(tasks::get_task)
                .put(tasks::update_task)
                .delete(tasks::delete_task),
        )
        .route("/api/v1/tasks/{id}/complete", post(tasks::complete_task))
        .route("/api/v1/tasks/{id}/snooze", post(tasks::snooze_task))
        .route(
            "/api/v1/assets",
            get(assets::list_assets).post(assets::create_asset),
        )
        .route(
            "/api/v1/assets/{id}",
            get(assets::get_asset)
                .put(assets::update_asset)
                .delete(assets::delete_asset),
        )
        .route(
            "/api/v1/supplies",
            get(supplies::list_supplies).post(supplies::create_supply),
        )
        .route(
            "/api/v1/supplies/{id}",
            get(supplies::get_supply)
                .put(supplies::update_supply)
                .delete(supplies::delete_supply),
        )
        .route(
            "/api/v1/locations",
            get(assets::placeholder).post(assets::placeholder),
        )
        .route(
            "/api/v1/log-entries",
            get(log::list_entries).post(log::create_entry),
        )
        .route("/api/v1/stats/costs", get(stats::costs))
        .layer(from_fn(api_auth_middleware))
        .with_state(state)
}

async fn api_auth_middleware(req: Request, next: Next) -> Result<Response, AppError> {
    let mut ui_ok = false;
    let mut api_ok = false;
    if let Some(h) = req.headers().get(header::COOKIE)
        && let Ok(s) = h.to_str()
    {
        ui_ok = s.split(';').any(|c| c.trim().starts_with("session="));
    }
    if let Some(h) = req.headers().get(header::AUTHORIZATION)
        && let Ok(s) = h.to_str()
        && let Some(token) = CONFIG.api_token.as_ref()
        && s == format!("Bearer {}", token)
    {
        api_ok = true;
    }
    if CONFIG.auth_mode == crate::config::AuthMode::None {
        api_ok = true;
        ui_ok = true;
    }
    if !ui_ok && !api_ok {
        return Err(AppError::Unauthorized);
    }
    Ok(next.run(req).await)
}

mod api;
mod config;
mod db;
mod domain;
mod error;
mod notify;
mod repo;
mod templates;
mod web;

use crate::config::CONFIG;
use crate::db::init_pool;
use axum::{Router, serve};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    info!(
        "starting home-maintenance v{} on port {} with timezone {}",
        config::VERSION,
        CONFIG.port,
        CONFIG.app_timezone
    );

    let db = init_pool().await?;
    info!("database backend: {}", db.backend.kind());

    let app = Router::new()
        .merge(web::ui_router(db.clone()).await)
        .merge(api::api_router(db.clone()))
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        .fallback(web::not_found);

    notify::spawn_digest_job(db).await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], CONFIG.port));
    let listener = TcpListener::bind(addr).await?;
    info!("listening on http://{}", addr);
    serve(listener, app).await?;
    Ok(())
}

fn init_logging() {
    let fmt = CONFIG.log_format.to_ascii_lowercase();
    if fmt == "json" {
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("home_maintenance=info".parse().unwrap()),
            )
            .finish();
        tracing::subscriber::set_global_default(subscriber).ok();
    } else {
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("home_maintenance=info".parse().unwrap()),
            )
            .finish();
        tracing::subscriber::set_global_default(subscriber).ok();
    }
}

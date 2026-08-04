use crate::error::AppError;
use crate::repo::assets::{self, AssetInput};
use crate::repo::attachments::list_attachments_for_owner;
use crate::repo::locations;
use crate::repo::log::list_log_entries;
use crate::repo::tasks::list_tasks;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::response::Redirect;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AssetListQuery {
    category: Option<String>,
    archived: Option<String>,
}

#[derive(Template)]
#[template(path = "assets.html")]
pub struct AssetListView {
    pub title: String,
    pub assets: Vec<assets::Asset>,
    pub locations: Vec<locations::Location>,
    pub categories: Vec<String>,
    pub selected_category: Option<String>,
    pub show_archived: bool,
}

pub async fn list_assets(
    State(state): State<AppState>,
    Query(q): Query<AssetListQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let all = assets::list_assets(&state.db).await?;
    let show_archived = q.archived.as_deref() == Some("1");

    let filtered: Vec<_> = all
        .into_iter()
        .filter(|a| {
            let cat_ok = q
                .category
                .as_ref()
                .map(|c| &a.category == c)
                .unwrap_or(true);
            let archived_ok = show_archived || a.archived == 0;
            cat_ok && archived_ok
        })
        .collect();

    let mut categories: Vec<String> = filtered.iter().map(|a| a.category.clone()).collect();
    categories.sort();
    categories.dedup();

    let locations = locations::list_locations(&state.db).await?;

    AssetListView {
        title: "Assets".to_string(),
        assets: filtered,
        locations,
        categories,
        selected_category: q.category,
        show_archived,
    }
    .render_response()
}

#[derive(Template)]
#[template(path = "asset_form.html")]
pub struct AssetFormView {
    pub title: String,
    pub locations: Vec<locations::Location>,
    pub categories: Vec<String>,
    pub asset: Option<assets::Asset>,
}

pub async fn new_asset_form(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let locations = locations::list_locations(&state.db).await?;
    AssetFormView {
        title: "New Asset".to_string(),
        locations,
        categories: default_categories(),
        asset: None,
    }
    .render_response()
}

pub async fn edit_asset_form(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let asset = assets::get_asset(&state.db, &id).await?;
    let locations = locations::list_locations(&state.db).await?;
    AssetFormView {
        title: format!("Edit {}", asset.name),
        locations,
        categories: default_categories(),
        asset: Some(asset),
    }
    .render_response()
}

#[derive(Deserialize)]
pub struct AssetFormPayload {
    name: String,
    location_id: Option<String>,
    category: String,
    make: Option<String>,
    model: Option<String>,
    serial: Option<String>,
    install_date: Option<String>,
    warranty_end: Option<String>,
    notes: Option<String>,
}

pub async fn create_asset(
    State(state): State<AppState>,
    axum::extract::Form(payload): axum::extract::Form<AssetFormPayload>,
) -> Result<Redirect, AppError> {
    let id = uuid::Uuid::now_v7().to_string();
    let input = AssetInput {
        name: payload.name,
        location_id: payload.location_id,
        category: payload.category,
        make: payload.make,
        model: payload.model,
        serial: payload.serial,
        install_date: payload.install_date,
        warranty_end: payload.warranty_end,
        notes: payload.notes,
    };
    assets::create_asset(&state.db, &id, input).await?;
    Ok(Redirect::to(&format!("/assets/{}", id)))
}

pub async fn update_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Form(payload): axum::extract::Form<AssetFormPayload>,
) -> Result<Redirect, AppError> {
    let input = AssetInput {
        name: payload.name,
        location_id: payload.location_id,
        category: payload.category,
        make: payload.make,
        model: payload.model,
        serial: payload.serial,
        install_date: payload.install_date,
        warranty_end: payload.warranty_end,
        notes: payload.notes,
    };
    assets::update_asset(&state.db, &id, input).await?;
    Ok(Redirect::to(&format!("/assets/{}", id)))
}

#[derive(Template)]
#[template(path = "asset_detail.html")]
pub struct AssetDetailView {
    pub title: String,
    pub asset: assets::Asset,
    pub location: Option<locations::Location>,
    pub tasks: Vec<TaskSummary>,
    pub log_entries: Vec<log_summary::LogEntrySummary>,
    pub attachments: Vec<attachment_summary::AttachmentSummary>,
    pub total_cost_cents: i64,
}

pub struct TaskSummary {
    pub id: String,
    pub name: String,
    pub schedule: String,
}

mod log_summary {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct LogEntrySummary {
        pub id: String,
        pub completed_date: String,
        pub kind: String,
        pub cost_cents: Option<i64>,
        pub vendor: Option<String>,
        pub notes: Option<String>,
    }
}

mod attachment_summary {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct AttachmentSummary {
        pub id: String,
        pub filename: String,
        pub content_type: String,
        pub byte_size: i64,
        pub caption: Option<String>,
    }
}

pub async fn view_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let asset = assets::get_asset(&state.db, &id).await?;

    let locations = locations::list_locations(&state.db).await?;
    let location = locations.into_iter().find(|l| {
        asset
            .location_id
            .as_ref()
            .map(|aid| aid == &l.id)
            .unwrap_or(false)
    });

    let tasks = list_tasks(&state.db)
        .await?
        .into_iter()
        .filter(|t| t.asset_id.as_ref() == Some(&id))
        .map(|t| TaskSummary {
            id: t.id,
            name: t.name,
            schedule: crate::domain::schedule::describe_schedule(
                &t.schedule_mode,
                t.interval_value,
                t.interval_unit.as_deref(),
                t.season_anchor.as_deref(),
                t.fixed_interval_years,
            ),
        })
        .collect();

    let log_rows = list_log_entries(&state.db, Some(&id), None, None, None, None, None).await?;
    let mut total_cost_cents = 0i64;
    let log_entries: Vec<_> = log_rows
        .into_iter()
        .map(|e| {
            total_cost_cents += e.cost_cents.unwrap_or(0);
            log_summary::LogEntrySummary {
                id: e.id,
                completed_date: e.completed_date,
                kind: e.kind,
                cost_cents: e.cost_cents,
                vendor: e.vendor,
                notes: e.notes,
            }
        })
        .collect();

    let attachments = list_attachments_for_owner(&state.db, "asset", &id)
        .await?
        .into_iter()
        .map(|a| attachment_summary::AttachmentSummary {
            id: a.id,
            filename: a.filename,
            content_type: a.content_type,
            byte_size: a.byte_size,
            caption: a.caption,
        })
        .collect();

    AssetDetailView {
        title: asset.name.clone(),
        asset,
        location,
        tasks,
        log_entries,
        attachments,
        total_cost_cents,
    }
    .render_response()
}

pub async fn archive_asset(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    assets::archive_asset(&state.db, &id).await?;
    Ok(Redirect::to("/assets"))
}

fn default_categories() -> Vec<String> {
    vec![
        "appliance".to_string(),
        "hvac".to_string(),
        "plumbing".to_string(),
        "electrical".to_string(),
        "exterior".to_string(),
        "landscaping".to_string(),
        "vehicle".to_string(),
        "other".to_string(),
    ]
}

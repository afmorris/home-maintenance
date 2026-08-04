use crate::error::AppError;
use crate::repo::supplies::{self, SupplyInput};
use crate::repo::tasks::list_tasks;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::{Path, State};
use axum::response::Redirect;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "supplies.html")]
pub struct SupplyListView {
    pub title: String,
    pub supplies: Vec<SupplyRow>,
}

pub struct SupplyRow {
    pub id: String,
    pub name: String,
    pub spec: Option<String>,
    #[allow(dead_code)]
    pub purchase_url: Option<String>,
    #[allow(dead_code)]
    pub notes: Option<String>,
    pub task_count: usize,
}

pub async fn list_supplies(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let supplies = supplies::list_supplies(&state.db).await?;
    let tasks = list_tasks(&state.db).await?;

    // Count how many tasks reference each supply via task_supplies.
    let counts: std::collections::HashMap<String, usize> = {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT supply_id, COUNT(*) AS c FROM task_supplies GROUP BY supply_id",
        )
        .fetch_all(&state.db.pool)
        .await?;
        rows.into_iter().map(|(id, c)| (id, c as usize)).collect()
    };

    let supply_rows: Vec<SupplyRow> = supplies
        .into_iter()
        .map(|s| SupplyRow {
            id: s.id.clone(),
            name: s.name,
            spec: s.spec,
            purchase_url: s.purchase_url,
            notes: s.notes,
            task_count: counts.get(&s.id).copied().unwrap_or(0),
        })
        .collect();

    // silence unused warning until detail page uses tasks
    let _ = tasks;

    SupplyListView {
        title: "Supplies".to_string(),
        supplies: supply_rows,
    }
    .render_response()
}

#[derive(Deserialize)]
pub struct SupplyFormPayload {
    name: String,
    spec: Option<String>,
    purchase_url: Option<String>,
    notes: Option<String>,
}

pub async fn create_supply(
    State(state): State<AppState>,
    axum::extract::Form(payload): axum::extract::Form<SupplyFormPayload>,
) -> Result<Redirect, AppError> {
    let id = uuid::Uuid::now_v7().to_string();
    let input = SupplyInput {
        name: payload.name,
        spec: payload.spec,
        purchase_url: payload.purchase_url,
        notes: payload.notes,
    };
    supplies::create_supply(&state.db, &id, input).await?;
    Ok(Redirect::to("/supplies"))
}

pub async fn delete_supply(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    supplies::delete_supply(&state.db, &id).await?;
    Ok(Redirect::to("/supplies"))
}

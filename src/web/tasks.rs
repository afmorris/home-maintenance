use crate::error::AppError;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::extract::State;

#[derive(Template)]
#[template(path = "tasks.html")]
pub struct TaskListView {
    pub title: String,
}

pub async fn list_tasks(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    TaskListView {
        title: "Tasks".to_string(),
    }
    .render_response()
}

#[derive(Template)]
#[template(path = "task_form.html")]
pub struct TaskFormView {
    pub title: String,
}

pub async fn new_task_form(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    TaskFormView {
        title: "New Task".to_string(),
    }
    .render_response()
}

pub async fn edit_task_form(
    State(_state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    TaskFormView {
        title: "Edit Task".to_string(),
    }
    .render_response()
}

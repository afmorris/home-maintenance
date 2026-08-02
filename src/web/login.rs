use crate::config::{AuthMode, CONFIG};
use crate::error::AppError;
use crate::web::{AppState, RenderTemplate};
use askama::Template;
use axum::Form;
use axum::extract::State;
use axum::response::{IntoResponse, Redirect, Response};
use cookie::time::{Duration, OffsetDateTime};
use cookie::{Cookie, CookieJar, SameSite};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginView;

pub async fn login_page(State(_state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    if CONFIG.auth_mode == AuthMode::None {
        return Err(AppError::BadRequest("auth is disabled".to_string()));
    }
    LoginView.render_response()
}

#[derive(serde::Deserialize, Debug)]
pub struct LoginForm {
    password: String,
}

pub async fn login_submit(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    let expected = CONFIG.app_password.as_ref().ok_or(AppError::Unauthorized)?;
    if form.password != *expected {
        return Err(AppError::Unauthorized);
    }
    let mut jar = CookieJar::new();
    jar.signed_mut(&state.session_key).add(
        Cookie::build(("session", "authenticated"))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .expires(OffsetDateTime::now_utc().checked_add(Duration::days(30)))
            .secure(false),
    );
    let cookie = jar.get("session").expect("just added");
    let mut resp = Redirect::to("/").into_response();
    resp.headers_mut().append(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );
    Ok(resp)
}

pub async fn logout(State(state): State<AppState>) -> Result<Response, AppError> {
    let mut jar = CookieJar::new();
    jar.signed_mut(&state.session_key).add(
        Cookie::build(("session", ""))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .max_age(Duration::seconds(0)),
    );
    let cookie = jar.get("session").expect("just added");
    let mut resp = Redirect::to("/login").into_response();
    resp.headers_mut().append(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );
    Ok(resp)
}

use once_cell::sync::Lazy;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::warn;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn default_port() -> u16 {
    8080
}

fn default_data_dir() -> String {
    "./data".to_string()
}

fn default_timezone() -> String {
    "America/New_York".to_string()
}

fn default_log_format() -> String {
    "text".to_string()
}

fn default_auth_mode() -> String {
    "none".to_string()
}

fn default_digest_hour() -> i64 {
    8
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppConfig {
    pub port: u16,
    pub database_url: Option<String>,
    pub data_dir: PathBuf,
    pub app_timezone: String,
    pub log_format: String,
    pub auth_mode: AuthMode,
    pub app_password: Option<String>,
    pub session_secret: String,
    pub ntfy_url: Option<String>,
    pub ntfy_topic: Option<String>,
    pub ntfy_token: Option<String>,
    pub digest_hour: i64,
    pub api_token: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    None,
    Password,
}

impl FromStr for AuthMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "none" => Ok(AuthMode::None),
            "password" => Ok(AuthMode::Password),
            _ => Err(format!("unknown auth mode: {}", s)),
        }
    }
}

pub fn base64_encode(bytes: [u8; 64]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
    let auth_mode_str = env::var("AUTH_MODE").unwrap_or_else(|_| default_auth_mode());
    let auth_mode = AuthMode::from_str(&auth_mode_str).expect("Invalid AUTH_MODE");
    let data_dir = env::var("DATA_DIR").unwrap_or_else(|_| default_data_dir());
    let data_dir = PathBuf::from(data_dir);

    let app_password = env::var("APP_PASSWORD").ok().filter(|s| !s.is_empty());
    if auth_mode == AuthMode::Password && app_password.is_none() {
        panic!("AUTH_MODE=password requires APP_PASSWORD to be set");
    }

    if auth_mode == AuthMode::None {
        warn!(
            "AUTH_MODE=none: the app is unauthenticated. Only run on a trusted network or behind an authenticating reverse proxy."
        );
    }

    let port = env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(default_port);
    let digest_hour = env::var("DIGEST_HOUR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(default_digest_hour);

    AppConfig {
        port,
        database_url,
        data_dir,
        app_timezone: env::var("APP_TIMEZONE").unwrap_or_else(|_| default_timezone()),
        log_format: env::var("LOG_FORMAT").unwrap_or_else(|_| default_log_format()),
        auth_mode,
        app_password,
        session_secret: env::var("SESSION_SECRET").unwrap_or_else(|_| {
            let mut buf = [0u8; 64];
            getrandom::fill(&mut buf).expect("session secret generation failed");
            base64_encode(buf)
        }),
        ntfy_url: env::var("NTFY_URL").ok().filter(|s| !s.is_empty()),
        ntfy_topic: env::var("NTFY_TOPIC").ok().filter(|s| !s.is_empty()),
        ntfy_token: env::var("NTFY_TOKEN").ok().filter(|s| !s.is_empty()),
        digest_hour,
        api_token: env::var("API_TOKEN").ok().filter(|s| !s.is_empty()),
    }
});

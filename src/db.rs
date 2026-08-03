use crate::config::CONFIG;
use sqlx::AssertSqlSafe;
use sqlx::Error as SqlxError;
use sqlx::Pool;
use sqlx::any::{Any, AnyConnectOptions, AnyPoolOptions, install_default_drivers};
use sqlx::migrate::Migrator;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Connection descriptor: PostgreSQL or SQLite.
#[derive(Debug, Clone)]
pub enum DbBackend {
    Postgres,
    Sqlite,
}

impl DbBackend {
    pub fn kind(&self) -> &'static str {
        match self {
            DbBackend::Postgres => "postgresql",
            DbBackend::Sqlite => "sqlite",
        }
    }

    #[allow(dead_code)]
    pub fn placeholder(&self, index: usize) -> String {
        // Runtime-query abstraction: Postgres uses $N, SQLite uses ?.
        match self {
            DbBackend::Postgres => format!("${}", index),
            DbBackend::Sqlite => "?".to_string(),
        }
    }

    pub fn returning(&self, columns: &[&str]) -> String {
        // Both Postgres and SQLite support RETURNING today; verified in CI.
        if columns.is_empty() {
            String::new()
        } else {
            format!(" RETURNING {}", columns.join(", "))
        }
    }
}

#[derive(Clone)]
pub struct Db {
    pub pool: Pool<Any>,
    pub backend: DbBackend,
}

pub async fn init_pool() -> Result<Db, SqlxError> {
    install_default_drivers();
    if let Some(url) = &CONFIG.database_url {
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
            info!("connecting to PostgreSQL");
            let opts = AnyConnectOptions::from_str(url)?;
            let pool = AnyPoolOptions::new()
                .max_connections(10)
                .acquire_timeout(Duration::from_secs(5))
                .connect_with(opts)
                .await?;
            MIGRATOR.run(&pool).await?;
            return Ok(Db {
                pool,
                backend: DbBackend::Postgres,
            });
        }
    }

    // SQLite path.
    let data_dir = &CONFIG.data_dir;
    std::fs::create_dir_all(data_dir).map_err(|e| {
        SqlxError::Io(std::io::Error::other(format!(
            "failed to create DATA_DIR: {}",
            e
        )))
    })?;
    let db_path = data_dir.join("home-maintenance.db");
    // Pre-create the empty file so sqlx AnyPool can open the SQLite database
    // when a relative path is used.
    std::fs::File::create(&db_path).map_err(|e| {
        SqlxError::Io(std::io::Error::other(format!(
            "failed to create SQLite database file: {}",
            e
        )))
    })?;
    let url = format!("sqlite:{}", db_path.display());
    warn!(
        "DATABASE_URL not set or not PostgreSQL; using SQLite at {}",
        db_path.display()
    );
    let opts = AnyConnectOptions::from_str(&url)?;
    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(opts)
        .await?;

    // SQLite pragmas.
    sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA busy_timeout = 5000;")
        .execute(&pool)
        .await?;

    MIGRATOR.run(&pool).await?;
    Ok(Db {
        pool,
        backend: DbBackend::Sqlite,
    })
}

/// Rewrite a query string using `$1, $2, ...` placeholders to the dialect
/// appropriate for the backend. All repository SQL is authored for Postgres
/// ($N). For SQLite this helper rewrites placeholders to `?`.
///
/// Returns a `Cow<'static, str>` so that in the Postgres case we can return
/// the original literal with `'static` lifetime (no allocation) while in the
/// SQLite case we return an owned `String` that lives for `'static`.
#[allow(dead_code)]
pub fn bind_sql(sql: &'static str, backend: &DbBackend) -> std::borrow::Cow<'static, str> {
    match backend {
        DbBackend::Postgres => std::borrow::Cow::Borrowed(sql),
        DbBackend::Sqlite => {
            let mut result = String::with_capacity(sql.len());
            let mut chars = sql.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '$' && chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                        chars.next();
                    }
                    result.push('?');
                } else {
                    result.push(ch);
                }
            }
            std::borrow::Cow::Owned(result)
        }
    }
}

/// Convenience: create a query builder with placeholder-aware SQL.
#[allow(dead_code)]
pub fn query(
    sql: &'static str,
    db: &Db,
) -> sqlx::query::Query<'static, Any, sqlx::any::AnyArguments> {
    sqlx::query(AssertSqlSafe(bind_sql(sql, &db.backend)))
}

#[allow(dead_code)]
pub fn query_as<O>(
    sql: &'static str,
    db: &Db,
) -> sqlx::query::QueryAs<'static, Any, O, sqlx::any::AnyArguments>
where
    O: for<'r> sqlx::FromRow<'r, sqlx::any::AnyRow>,
{
    sqlx::query_as(AssertSqlSafe(bind_sql(sql, &db.backend)))
}

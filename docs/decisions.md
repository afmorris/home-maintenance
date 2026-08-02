# Decision Log

## Database driver registration for `sqlx::AnyPool`

`AnyPool` requires drivers to be installed explicitly with `install_default_drivers()` before connecting. We call this once in `db::init_pool` so the same binary works with both PostgreSQL and SQLite backends.

## SQLite database file creation

`sqlx::AnyPool` with a `sqlite:<path>` URL fails with "unable to open database file" if the database file does not already exist. To keep the dev experience zero-touch, we pre-create an empty file with `std::fs::File::create` after ensuring `DATA_DIR` exists. This only applies to SQLite; PostgreSQL connections use the provided `DATABASE_URL` directly.

## Middleware auth/logging

We use `axum::middleware::from_fn_with_state` and a single `AppState` type that implements `FromRef<AppState>` for `Key` and `Arc<Db>`, and `FromRequestParts<AppState>` for itself. This lets the middleware receive the full `AppState` while handlers can use `State<AppState>`, `State<Key>`, or `State<Arc<Db>>` as needed.

## Placeholder strategy

All repository SQL is authored with PostgreSQL `$N` placeholders and passed through `db::bind_sql` when building dynamic queries. SQLite receives `?` placeholders in the same order. `RETURNING` is used only for simple single-row inserts and has been verified to work on both backends in current versions.

## Auth in v1

`AUTH_MODE=none` is the default. When enabled, the UI/API are open. The spec explicitly allows this for trusted networks or authenticating reverse proxies; we log a startup warning. `AUTH_MODE=password` uses a single shared password and a signed session cookie. We will not implement per-user accounts or OAuth in v1.

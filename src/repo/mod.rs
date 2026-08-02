// All SQL lives here, one module per aggregate. Placeholders are authored in
// Postgres `$N` style and rewritten to `?` for SQLite by db::query helpers.

pub mod assets;
pub mod locations;
pub mod log;
pub mod reminders;
pub mod supplies;
pub mod tags;
pub mod tasks;

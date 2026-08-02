use crate::db::Db;
use tracing::info;

pub async fn spawn_digest_job(_db: Db) -> anyhow::Result<()> {
    info!("notification digest job not yet implemented");
    Ok(())
}

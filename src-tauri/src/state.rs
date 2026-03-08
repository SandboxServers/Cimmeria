use cimmeria_services::database::DatabasePool;
use cimmeria_services::orchestrator::Orchestrator;
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;

pub struct AppState {
    pub orchestrator: Arc<Orchestrator>,
    pub repo_root: PathBuf,
    pub db_connection_string: String,
    db_pool: OnceCell<PgPool>,
}

impl AppState {
    pub fn new(orchestrator: Arc<Orchestrator>, db_connection_string: String) -> Self {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri should live under the repository root")
            .to_path_buf();

        Self {
            orchestrator,
            repo_root,
            db_connection_string,
            db_pool: OnceCell::new(),
        }
    }

    pub async fn db_pool(&self) -> Result<&PgPool, String> {
        self.db_pool
            .get_or_try_init(|| async {
                let db = DatabasePool::connect(&self.db_connection_string)
                    .await
                    .map_err(|error| format!("failed to connect to PostgreSQL: {error}"))?;

                if !db.health_check().await {
                    return Err("PostgreSQL health check failed".to_string());
                }

                Ok(db.pool().clone())
            })
            .await
    }
}

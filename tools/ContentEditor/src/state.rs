use crate::script::definitions::ScriptDefinitions;
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tokio::sync::{Mutex, OnceCell};

pub struct AppState {
    pub repo_root: PathBuf,
    pool: OnceCell<PgPool>,
    connection_string: Mutex<Option<String>>,
    server_url: Mutex<String>,
    script_defs: OnceLock<ScriptDefinitions>,
}

impl AppState {
    pub fn new() -> Self {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|tools| tools.parent())
            .expect("ContentEditor should live under tools/ in the repository root")
            .to_path_buf();

        Self {
            repo_root,
            pool: OnceCell::new(),
            connection_string: Mutex::new(None),
            server_url: Mutex::new("http://localhost:8443".to_string()),
            script_defs: OnceLock::new(),
        }
    }

    pub async fn connect(&self, connection_string: &str) -> Result<&PgPool, String> {
        // If already connected, return existing pool
        if let Some(pool) = self.pool.get() {
            return Ok(pool);
        }

        let pool = PgPool::connect(connection_string)
            .await
            .map_err(|e| format!("Failed to connect to PostgreSQL: {e}"))?;

        // Verify connection works
        sqlx::query("SELECT 1")
            .execute(&pool)
            .await
            .map_err(|e| format!("PostgreSQL health check failed: {e}"))?;

        *self.connection_string.lock().await = Some(connection_string.to_string());

        self.pool
            .set(pool)
            .map_err(|_| "Database pool already initialized".to_string())?;

        Ok(self.pool.get().unwrap())
    }

    pub fn pool(&self) -> Result<&PgPool, String> {
        self.pool
            .get()
            .ok_or_else(|| "Not connected to database".to_string())
    }

    pub async fn server_url(&self) -> String {
        self.server_url.lock().await.clone()
    }

    pub async fn set_server_url(&self, url: String) {
        *self.server_url.lock().await = url;
    }

    pub fn seed_dir(&self) -> PathBuf {
        self.repo_root.join("db").join("resources").join("Content").join("Seed")
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.repo_root.join("data").join("scripts")
    }

    /// Lazy-load script definitions (Nodes.xml + enumerations.xml) on first access.
    pub fn script_definitions(&self) -> Result<&ScriptDefinitions, String> {
        if let Some(defs) = self.script_defs.get() {
            return Ok(defs);
        }
        let defs = crate::script::definitions::load_definitions(&self.repo_root)
            .map_err(|e| format!("Failed to load script definitions: {e}"))?;
        // If another thread beat us, that's fine -- just return whichever won
        let _ = self.script_defs.set(defs);
        Ok(self.script_defs.get().unwrap())
    }
}

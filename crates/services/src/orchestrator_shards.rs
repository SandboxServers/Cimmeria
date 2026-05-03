//! Shard lookup helpers for the orchestrator.
//!
//! Reads shard rows from the `shards` table at startup so the auth service
//! can advertise them in Phase 1 responses.

use std::sync::Arc;

use sqlx::PgPool;

/// Row returned by [`query_all_shards`].
pub(crate) struct ShardRow {
    pub(crate) name: String,
    pub(crate) protected: bool,
}

/// Query all shards from the `shards` table, ordered by shard_id.
/// Returns a fallback single-entry list on error.
pub(crate) async fn query_all_shards(pool: &Arc<PgPool>) -> Vec<ShardRow> {
    #[derive(sqlx::FromRow)]
    struct DbShardRow {
        name: Option<String>,
        protected: bool,
    }

    match sqlx::query_as::<_, DbShardRow>("SELECT name, protected FROM shards ORDER BY shard_id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(rows) => {
            let shards: Vec<ShardRow> = rows
                .into_iter()
                .filter_map(|r| {
                    r.name.map(|n| ShardRow {
                        name: n,
                        protected: r.protected,
                    })
                })
                .collect();
            if shards.is_empty() {
                tracing::error!("No shards found in database — using fallback name 'Shard'. Run: INSERT INTO shards (shard_id, name, key, protected) VALUES (1, 'Test', '', false);");
                vec![ShardRow {
                    name: "Shard".to_string(),
                    protected: false,
                }]
            } else {
                tracing::info!(
                    count = shards.len(),
                    "Loaded shards from database: {:?}",
                    shards.iter().map(|s| &s.name).collect::<Vec<_>>()
                );
                shards
            }
        }
        Err(e) => {
            tracing::error!("Failed to query shards table: {e} — using fallback name 'Shard'. Ensure the 'shards' table exists (re-run Initialize-CimmeriaDatabase -Force).");
            vec![ShardRow {
                name: "Shard".to_string(),
                protected: false,
            }]
        }
    }
}

//! Content engine construction — builds a [`ChainEngine`] from chains stored
//! in the database.
//!
//! All chain data lives in the `resources.content_*` tables. Startup loads
//! every enabled chain; if the DB is unavailable or the tables are missing,
//! we return an empty engine and the server runs without content scripting.

use sqlx::PgPool;

use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::loader::{
    DbActionRow, DbChainRow, DbConditionRow, DbTriggerRow, build_chains_from_rows,
};

/// Build the content engine by loading chains from the database.
///
/// Returns an empty engine if the DB pool is unavailable or the content
/// tables don't exist yet — all chain data lives in the database.
pub async fn build_engine(db_pool: Option<&PgPool>) -> ChainEngine {
    if let Some(pool) = db_pool {
        match load_chains_from_db(pool).await {
            Ok(chains) => {
                let mut engine = ChainEngine::new();
                for chain in chains {
                    engine.register_chain(chain);
                }
                tracing::info!(chains = engine.chain_count(), "Content engine loaded from database");
                return engine;
            }
            Err(e) => {
                tracing::error!("Failed to load content chains from DB: {e} — content engine will be empty");
            }
        }
    } else {
        tracing::warn!("No DB pool available — content engine will be empty");
    }

    ChainEngine::new()
}

/// Load all enabled content chains from the database.
async fn load_chains_from_db(pool: &PgPool) -> Result<Vec<Chain>, sqlx::Error> {
    use sqlx::Row;

    let chain_rows: Vec<DbChainRow> = sqlx::query(
        "SELECT chain_id, description, scope_type, scope_id, enabled, priority \
         FROM resources.content_chains ORDER BY chain_id"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbChainRow {
        chain_id: r.get("chain_id"),
        description: r.get("description"),
        scope_type: r.get("scope_type"),
        scope_id: r.get("scope_id"),
        enabled: r.get("enabled"),
        priority: r.get("priority"),
    })
    .collect();

    let trigger_rows: Vec<DbTriggerRow> = sqlx::query(
        "SELECT chain_id, event_type, event_key, scope, once, sort_order \
         FROM resources.content_triggers ORDER BY chain_id, sort_order"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbTriggerRow {
        chain_id: r.get("chain_id"),
        event_type: r.get("event_type"),
        event_key: r.get("event_key"),
        scope: r.get("scope"),
        once: r.get("once"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let condition_rows: Vec<DbConditionRow> = sqlx::query(
        "SELECT chain_id, condition_type, target_id, target_key, operator, value, sort_order \
         FROM resources.content_conditions ORDER BY chain_id, sort_order"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbConditionRow {
        chain_id: r.get("chain_id"),
        condition_type: r.get("condition_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        operator: r.get("operator"),
        value: r.get("value"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let action_rows: Vec<DbActionRow> = sqlx::query(
        "SELECT chain_id, action_type, target_id, target_key, params, delay_ms, sort_order \
         FROM resources.content_actions ORDER BY chain_id, sort_order"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbActionRow {
        chain_id: r.get("chain_id"),
        action_type: r.get("action_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        params: r.get("params"),
        delay_ms: r.get("delay_ms"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    tracing::info!(
        chains = chain_rows.len(),
        triggers = trigger_rows.len(),
        conditions = condition_rows.len(),
        actions = action_rows.len(),
        "Loaded content engine rows from database"
    );

    Ok(build_chains_from_rows(chain_rows, trigger_rows, condition_rows, action_rows))
}

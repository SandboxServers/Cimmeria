use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;

#[derive(Serialize)]
pub struct SpaceEntry {
    pub space_id: String,
    pub chain_count: i64,
}

#[derive(Serialize)]
pub struct ChainRow {
    pub chain_id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub scope_type: String,
    pub scope_id: Option<i32>,
    pub enabled: bool,
    pub priority: i32,
    pub editor_data: Value,
    pub triggers: Vec<TriggerRow>,
    pub conditions: Vec<ConditionRow>,
    pub actions: Vec<ActionRow>,
    pub counters: Vec<CounterRow>,
}

#[derive(Serialize)]
pub struct TriggerRow {
    pub trigger_id: i64,
    pub event_type: String,
    pub event_key: String,
    pub scope: String,
    pub once: bool,
    pub sort_order: i32,
    pub editor_data: Value,
}

#[derive(Serialize)]
pub struct ConditionRow {
    pub condition_id: i64,
    pub condition_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub operator: String,
    pub value: String,
    pub sort_order: i32,
    pub editor_data: Value,
}

#[derive(Serialize)]
pub struct ActionRow {
    pub action_id: i64,
    pub action_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub params: Value,
    pub delay_ms: i32,
    pub sort_order: i32,
    pub editor_data: Value,
}

#[derive(Serialize)]
pub struct CounterRow {
    pub counter_id: i64,
    pub counter_name: String,
    pub target_value: i32,
    pub reset_on: String,
    pub editor_data: Value,
}

#[derive(Deserialize)]
pub struct SaveChainInput {
    pub chain_id: Option<i64>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub scope_type: String,
    pub scope_id: Option<i32>,
    pub enabled: bool,
    pub priority: i32,
    pub editor_data: Value,
    pub triggers: Vec<SaveTriggerInput>,
    pub conditions: Vec<SaveConditionInput>,
    pub actions: Vec<SaveActionInput>,
    pub counters: Vec<SaveCounterInput>,
}

#[derive(Deserialize)]
pub struct SaveTriggerInput {
    pub event_type: String,
    pub event_key: String,
    pub scope: String,
    pub once: bool,
    pub sort_order: i32,
    pub editor_data: Value,
}

#[derive(Deserialize)]
pub struct SaveConditionInput {
    pub condition_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub operator: String,
    pub value: String,
    pub sort_order: i32,
    pub editor_data: Value,
}

#[derive(Deserialize)]
pub struct SaveActionInput {
    pub action_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub params: Value,
    pub delay_ms: i32,
    pub sort_order: i32,
    pub editor_data: Value,
}

#[derive(Deserialize)]
pub struct SaveCounterInput {
    pub counter_name: String,
    pub target_value: i32,
    pub reset_on: String,
    pub editor_data: Value,
}

#[tauri::command]
pub async fn list_spaces(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SpaceEntry>, String> {
    tracing::debug!("list_spaces called");
    let pool = state.pool()?;

    // Debug: check what search_path is on this connection
    let sp: (String,) = sqlx::query_as("SELECT current_setting('search_path')")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to query search_path: {e}"))?;
    tracing::debug!("list_spaces: search_path = {:?}", sp.0);

    let rows = sqlx::query(
        r#"
        SELECT
            editor_data ->> 'spaceId' AS space_id,
            COUNT(*) AS chain_count
        FROM content_chains
        WHERE editor_data ->> 'spaceId' IS NOT NULL
        GROUP BY editor_data ->> 'spaceId'
        ORDER BY editor_data ->> 'spaceId'
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list spaces: {e}");
        format!("Failed to list spaces: {e}")
    })?;

    tracing::debug!("list_spaces returned {} rows", rows.len());
    Ok(rows
        .iter()
        .map(|row| SpaceEntry {
            space_id: row.get("space_id"),
            chain_count: row.get("chain_count"),
        })
        .collect())
}

#[tauri::command]
pub async fn load_chains(
    state: tauri::State<'_, AppState>,
    space_id: String,
    mission_id: Option<String>,
) -> Result<Vec<ChainRow>, String> {
    let pool = state.pool()?;

    let chain_rows = sqlx::query(
        r#"
        SELECT chain_id, name, description, scope_type, scope_id, enabled, priority, editor_data
        FROM content_chains
        WHERE editor_data ->> 'spaceId' = $1
          AND ($2 IS NULL OR editor_data ->> 'missionId' = $2)
        ORDER BY priority DESC, chain_id ASC
        "#,
    )
    .bind(&space_id)
    .bind(&mission_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load chains: {e}"))?;

    let mut chains = Vec::new();

    for row in chain_rows {
        let chain_id: i64 = row.get("chain_id");

        let triggers = sqlx::query(
            "SELECT trigger_id, event_type, event_key, scope, once, sort_order, editor_data \
             FROM content_triggers WHERE chain_id = $1 ORDER BY sort_order",
        )
        .bind(chain_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load triggers: {e}"))?
        .iter()
        .map(|r| TriggerRow {
            trigger_id: r.get("trigger_id"),
            event_type: r.get("event_type"),
            event_key: r.get("event_key"),
            scope: r.get("scope"),
            once: r.get("once"),
            sort_order: r.get("sort_order"),
            editor_data: r.get("editor_data"),
        })
        .collect();

        let conditions = sqlx::query(
            "SELECT condition_id, condition_type, target_id, target_key, operator, value, sort_order, editor_data \
             FROM content_conditions WHERE chain_id = $1 ORDER BY sort_order",
        )
        .bind(chain_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load conditions: {e}"))?
        .iter()
        .map(|r| ConditionRow {
            condition_id: r.get("condition_id"),
            condition_type: r.get("condition_type"),
            target_id: r.get("target_id"),
            target_key: r.get("target_key"),
            operator: r.get("operator"),
            value: r.get("value"),
            sort_order: r.get("sort_order"),
            editor_data: r.get("editor_data"),
        })
        .collect();

        let actions = sqlx::query(
            "SELECT action_id, action_type, target_id, target_key, params, delay_ms, sort_order, editor_data \
             FROM content_actions WHERE chain_id = $1 ORDER BY sort_order",
        )
        .bind(chain_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load actions: {e}"))?
        .iter()
        .map(|r| ActionRow {
            action_id: r.get("action_id"),
            action_type: r.get("action_type"),
            target_id: r.get("target_id"),
            target_key: r.get("target_key"),
            params: r.get("params"),
            delay_ms: r.get("delay_ms"),
            sort_order: r.get("sort_order"),
            editor_data: r.get("editor_data"),
        })
        .collect();

        let counters = sqlx::query(
            "SELECT counter_id, counter_name, target_value, reset_on, editor_data \
             FROM content_counters WHERE chain_id = $1 ORDER BY counter_id",
        )
        .bind(chain_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load counters: {e}"))?
        .iter()
        .map(|r| CounterRow {
            counter_id: r.get("counter_id"),
            counter_name: r.get("counter_name"),
            target_value: r.get("target_value"),
            reset_on: r.get("reset_on"),
            editor_data: r.get("editor_data"),
        })
        .collect();

        chains.push(ChainRow {
            chain_id,
            name: row.get("name"),
            description: row.get("description"),
            scope_type: row.get("scope_type"),
            scope_id: row.get("scope_id"),
            enabled: row.get("enabled"),
            priority: row.get("priority"),
            editor_data: row.get("editor_data"),
            triggers,
            conditions,
            actions,
            counters,
        });
    }

    Ok(chains)
}

#[tauri::command]
pub async fn save_chains(
    state: tauri::State<'_, AppState>,
    chains: Vec<SaveChainInput>,
) -> Result<Vec<i64>, String> {
    let pool = state.pool()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    let mut saved_ids = Vec::new();

    for chain in &chains {
        // Delete existing chain and children (cascade) if updating
        if let Some(existing_id) = chain.chain_id {
            sqlx::query("DELETE FROM content_chains WHERE chain_id = $1")
                .bind(existing_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to delete existing chain {existing_id}: {e}"))?;
        }

        // Insert chain
        let chain_id: i64 = if let Some(existing_id) = chain.chain_id {
            sqlx::query_scalar::<_, i64>(
                "INSERT INTO content_chains (chain_id, name, description, scope_type, scope_id, enabled, priority, editor_data) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING chain_id",
            )
            .bind(existing_id)
            .bind(&chain.name)
            .bind(&chain.description)
            .bind(&chain.scope_type)
            .bind(chain.scope_id)
            .bind(chain.enabled)
            .bind(chain.priority)
            .bind(&chain.editor_data)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert chain: {e}"))?
        } else {
            sqlx::query_scalar::<_, i64>(
                "INSERT INTO content_chains (name, description, scope_type, scope_id, enabled, priority, editor_data) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING chain_id",
            )
            .bind(&chain.name)
            .bind(&chain.description)
            .bind(&chain.scope_type)
            .bind(chain.scope_id)
            .bind(chain.enabled)
            .bind(chain.priority)
            .bind(&chain.editor_data)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert chain: {e}"))?
        };

        for trigger in &chain.triggers {
            sqlx::query(
                "INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order, editor_data) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(chain_id)
            .bind(&trigger.event_type)
            .bind(&trigger.event_key)
            .bind(&trigger.scope)
            .bind(trigger.once)
            .bind(trigger.sort_order)
            .bind(&trigger.editor_data)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert trigger: {e}"))?;
        }

        for condition in &chain.conditions {
            sqlx::query(
                "INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order, editor_data) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(chain_id)
            .bind(&condition.condition_type)
            .bind(condition.target_id)
            .bind(&condition.target_key)
            .bind(&condition.operator)
            .bind(&condition.value)
            .bind(condition.sort_order)
            .bind(&condition.editor_data)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert condition: {e}"))?;
        }

        for action in &chain.actions {
            sqlx::query(
                "INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order, editor_data) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(chain_id)
            .bind(&action.action_type)
            .bind(action.target_id)
            .bind(&action.target_key)
            .bind(&action.params)
            .bind(action.delay_ms)
            .bind(action.sort_order)
            .bind(&action.editor_data)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert action: {e}"))?;
        }

        for counter in &chain.counters {
            sqlx::query(
                "INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on, editor_data) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(chain_id)
            .bind(&counter.counter_name)
            .bind(counter.target_value)
            .bind(&counter.reset_on)
            .bind(&counter.editor_data)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert counter: {e}"))?;
        }

        saved_ids.push(chain_id);
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    tracing::info!("Saved {} chains to database", saved_ids.len());
    Ok(saved_ids)
}

#[tauri::command]
pub async fn delete_chain(
    state: tauri::State<'_, AppState>,
    chain_id: i64,
) -> Result<(), String> {
    let pool = state.pool()?;

    sqlx::query("DELETE FROM content_chains WHERE chain_id = $1")
        .bind(chain_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to delete chain {chain_id}: {e}"))?;

    tracing::info!("Deleted chain {chain_id}");
    Ok(())
}

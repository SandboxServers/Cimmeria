use std::collections::HashMap;

use crate::state::AppState;
use serde_json::{json, Map, Value};
use sqlx::{PgPool, Postgres, Row, Transaction};

const CREATE_CONTENT_CHAINS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS content_chains (
    chain_id BIGSERIAL PRIMARY KEY,
    name VARCHAR(200),
    scope_type VARCHAR(50) NOT NULL,
    scope_id INTEGER,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 0,
    description TEXT,
    editor_data JSONB NOT NULL DEFAULT '{}'::jsonb
)
"#;

const CREATE_CONTENT_CHAINS_SCOPE_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_chains_scope ON content_chains (scope_type, scope_id)
"#;

const CREATE_CONTENT_TRIGGERS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS content_triggers (
    trigger_id BIGSERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL REFERENCES content_chains(chain_id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    event_key VARCHAR(200) NOT NULL,
    scope VARCHAR(20) NOT NULL DEFAULT 'player',
    once BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    editor_data JSONB NOT NULL DEFAULT '{}'::jsonb
)
"#;

const CREATE_CONTENT_TRIGGERS_CHAIN_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_triggers_chain ON content_triggers (chain_id)
"#;

const CREATE_CONTENT_TRIGGERS_EVENT_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_triggers_event ON content_triggers (event_type, event_key)
"#;

const CREATE_CONTENT_CONDITIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS content_conditions (
    condition_id BIGSERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL REFERENCES content_chains(chain_id) ON DELETE CASCADE,
    condition_type VARCHAR(50) NOT NULL,
    target_id INTEGER,
    target_key VARCHAR(200),
    operator VARCHAR(10) NOT NULL DEFAULT 'eq',
    value VARCHAR(200) NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    editor_data JSONB NOT NULL DEFAULT '{}'::jsonb
)
"#;

const CREATE_CONTENT_CONDITIONS_CHAIN_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_conditions_chain ON content_conditions (chain_id)
"#;

const CREATE_CONTENT_ACTIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS content_actions (
    action_id BIGSERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL REFERENCES content_chains(chain_id) ON DELETE CASCADE,
    action_type VARCHAR(50) NOT NULL,
    target_id INTEGER,
    target_key VARCHAR(200),
    params JSONB NOT NULL DEFAULT '{}'::jsonb,
    delay_ms INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    editor_data JSONB NOT NULL DEFAULT '{}'::jsonb
)
"#;

const CREATE_CONTENT_ACTIONS_CHAIN_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_actions_chain ON content_actions (chain_id)
"#;

const CREATE_CONTENT_COUNTERS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS content_counters (
    counter_id BIGSERIAL PRIMARY KEY,
    chain_id BIGINT NOT NULL REFERENCES content_chains(chain_id) ON DELETE CASCADE,
    counter_name VARCHAR(100) NOT NULL,
    target_value INTEGER NOT NULL,
    reset_on VARCHAR(50) NOT NULL DEFAULT 'mission_complete',
    editor_data JSONB NOT NULL DEFAULT '{}'::jsonb
)
"#;

const CREATE_CONTENT_COUNTERS_CHAIN_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_counters_chain ON content_counters (chain_id)
"#;

const CREATE_CONTENT_EDITOR_SCOPES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS content_editor_scopes (
    scope_id BIGSERIAL PRIMARY KEY,
    space_id TEXT NOT NULL,
    mission_id TEXT NOT NULL DEFAULT '',
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT content_editor_scopes_unique_scope UNIQUE (space_id, mission_id)
)
"#;

#[tauri::command]
pub async fn load_chain_editor_content(
    state: tauri::State<'_, AppState>,
    space_id: String,
    mission_id: Option<String>,
) -> Result<Option<Value>, String> {
    let pool = state.db_pool().await?;
    ensure_content_schema(pool).await?;
    let normalized_mission_id = normalize_mission_id(mission_id);

    if let Some(payload) = sqlx::query_scalar::<_, Value>(
        r#"
        SELECT payload
        FROM content_editor_scopes
        WHERE space_id = $1 AND mission_id = $2
        "#,
    )
    .bind(&space_id)
    .bind(&normalized_mission_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| format!("failed to load chain editor content: {error}"))?
    {
        return Ok(Some(payload));
    }

    let chain_rows = sqlx::query(
        r#"
        SELECT chain_id, name, scope_type, scope_id, enabled, priority, description, editor_data
        FROM content_chains
        WHERE editor_data ->> 'spaceId' = $1
          AND ($2 = '' OR editor_data ->> 'missionId' = $2)
        ORDER BY priority DESC, chain_id ASC
        "#,
    )
    .bind(&space_id)
    .bind(&normalized_mission_id)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("failed to load content chains: {error}"))?;

    if chain_rows.is_empty() {
        return Ok(None);
    }

    Ok(Some(
        reconstruct_scope_payload(&space_id, &normalized_mission_id, chain_rows, pool).await?,
    ))
}

#[tauri::command]
pub async fn save_chain_editor_content(
    state: tauri::State<'_, AppState>,
    space_id: String,
    mission_id: Option<String>,
    payload: Value,
) -> Result<(), String> {
    let pool = state.db_pool().await?;
    ensure_content_schema(pool).await?;
    let normalized_mission_id = normalize_mission_id(mission_id);
    let parsed = parse_scope_payload(payload, &space_id, &normalized_mission_id)?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("failed to begin content save transaction: {error}"))?;

    delete_scope_content(&mut tx, &space_id, &normalized_mission_id).await?;

    for chain in &parsed.chains {
        let chain_id = insert_chain(&mut tx, chain).await?;

        for trigger in &chain.triggers {
            insert_trigger(&mut tx, chain_id, trigger).await?;
        }

        for condition in &chain.conditions {
            insert_condition(&mut tx, chain_id, condition).await?;
        }

        for action in &chain.actions {
            insert_action(&mut tx, chain_id, action).await?;
        }

        for counter in &chain.counters {
            insert_counter(&mut tx, chain_id, counter).await?;
        }
    }

    sqlx::query(
        r#"
        INSERT INTO content_editor_scopes (space_id, mission_id, payload)
        VALUES ($1, $2, $3)
        ON CONFLICT (space_id, mission_id)
        DO UPDATE SET
            payload = EXCLUDED.payload,
            updated_at = NOW()
        "#,
    )
    .bind(&space_id)
    .bind(&normalized_mission_id)
    .bind(&parsed.original_payload)
    .execute(&mut *tx)
    .await
    .map_err(|error| format!("failed to save chain editor scope metadata: {error}"))?;

    tx.commit()
        .await
        .map_err(|error| format!("failed to commit content save transaction: {error}"))?;

    Ok(())
}

#[tauri::command]
pub async fn validate_chain_editor_content(
    state: tauri::State<'_, AppState>,
    space_id: String,
    mission_id: Option<String>,
    payload: Value,
) -> Result<Value, String> {
    let pool = state.db_pool().await?;
    ensure_content_schema(pool).await?;
    let normalized_mission_id = normalize_mission_id(mission_id);
    let parsed = parse_scope_payload(payload, &space_id, &normalized_mission_id)?;

    Ok(json!({
        "ok": true,
        "chainCount": parsed.chains.len(),
        "triggerCount": parsed.chains.iter().map(|chain| chain.triggers.len()).sum::<usize>(),
        "conditionCount": parsed.chains.iter().map(|chain| chain.conditions.len()).sum::<usize>(),
        "actionCount": parsed.chains.iter().map(|chain| chain.actions.len()).sum::<usize>(),
        "counterCount": parsed.chains.iter().map(|chain| chain.counters.len()).sum::<usize>(),
    }))
}

async fn reconstruct_scope_payload(
    space_id: &str,
    mission_id: &str,
    chain_rows: Vec<sqlx::postgres::PgRow>,
    pool: &PgPool,
) -> Result<Value, String> {
    let mut nodes = Vec::new();
    let edges = Vec::<Value>::new();
    let mut selected_chain_id = String::new();

    for row in chain_rows {
        let chain_id: i64 = row.try_get("chain_id").map_err(sqlx_error)?;
        let editor_data = row
            .try_get::<Value, _>("editor_data")
            .map_err(sqlx_error)?
            .as_object()
            .cloned()
            .unwrap_or_default();
        let chain_node_id = editor_data
            .get("nodeId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("chain-{chain_id}"));

        if selected_chain_id.is_empty() {
            selected_chain_id = chain_node_id.clone();
        }

        nodes.push(json!({
            "id": chain_node_id,
            "position": editor_data.get("position").cloned().unwrap_or_else(|| json!({ "x": 0, "y": 0 })),
            "data": {
                "kind": "chain",
                "name": row.try_get::<Option<String>, _>("name").map_err(sqlx_error)?.unwrap_or_default(),
                "summary": row.try_get::<Option<String>, _>("description").map_err(sqlx_error)?.unwrap_or_default(),
                "scopeType": row.try_get::<String, _>("scope_type").map_err(sqlx_error)?,
                "scopeId": row.try_get::<Option<i32>, _>("scope_id").map_err(sqlx_error)?.map(|value| value.to_string()).unwrap_or_default(),
                "enabled": row.try_get::<bool, _>("enabled").map_err(sqlx_error)?,
                "priority": row.try_get::<i32, _>("priority").map_err(sqlx_error)?,
                "source": editor_data.get("source").and_then(Value::as_str).unwrap_or("content_chains"),
                "semantic": editor_data.get("semantic").and_then(Value::as_str).unwrap_or("Story"),
                "color": editor_data.get("color").and_then(Value::as_str).unwrap_or("#22c55e"),
                "spaceId": editor_data.get("spaceId").and_then(Value::as_str).unwrap_or(space_id),
                "missionId": editor_data.get("missionId").cloned().unwrap_or_else(|| {
                    if mission_id.is_empty() {
                        Value::Null
                    } else {
                        Value::String(mission_id.to_string())
                    }
                }),
                "counts": editor_data.get("counts").cloned().unwrap_or_else(|| json!({})),
                "sequenceCount": editor_data.get("sequenceCount").cloned().unwrap_or_else(|| json!(0))
            }
        }));

        nodes.extend(
            load_child_nodes(
                pool,
                chain_id,
                &chain_node_id,
                "content_triggers",
                "trigger",
            )
            .await?,
        );
        nodes.extend(
            load_child_nodes(
                pool,
                chain_id,
                &chain_node_id,
                "content_conditions",
                "condition",
            )
            .await?,
        );
        nodes.extend(
            load_child_nodes(pool, chain_id, &chain_node_id, "content_actions", "action").await?,
        );
        nodes.extend(
            load_child_nodes(
                pool,
                chain_id,
                &chain_node_id,
                "content_counters",
                "counter",
            )
            .await?,
        );
    }

    Ok(json!({
        "version": 1,
        "spaceId": space_id,
        "missionId": if mission_id.is_empty() { Value::Null } else { Value::String(mission_id.to_string()) },
        "nodes": nodes,
        "edges": edges,
        "selectedChainId": selected_chain_id,
        "selectedNodeId": Value::String(String::new()),
        "selectedSequenceId": Value::String(String::new()),
    }))
}

async fn load_child_nodes(
    pool: &PgPool,
    chain_id: i64,
    parent_id: &str,
    table_name: &str,
    family: &str,
) -> Result<Vec<Value>, String> {
    let order_clause = if table_name == "content_counters" {
        "counter_id ASC"
    } else {
        "sort_order ASC"
    };
    let query =
        format!("SELECT editor_data FROM {table_name} WHERE chain_id = $1 ORDER BY {order_clause}");
    let rows = sqlx::query(&query)
        .bind(chain_id)
        .fetch_all(pool)
        .await
        .map_err(|error| format!("failed to load {table_name} editor metadata: {error}"))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let editor_data = row
                .try_get::<Value, _>("editor_data")
                .unwrap_or_else(|_| json!({}));
            let node_id = editor_data
                .get("nodeId")
                .and_then(Value::as_str)
                .unwrap_or("restored-node")
                .to_string();

            json!({
                "id": node_id,
                "parentId": parent_id,
                "extent": "parent",
                "type": "missionNode",
                "position": editor_data.get("position").cloned().unwrap_or_else(|| json!({ "x": 0, "y": 0 })),
                "data": merge_editor_defaults(editor_data, family),
            })
        })
        .collect())
}

fn merge_editor_defaults(editor_data: Value, family: &str) -> Value {
    let mut map = editor_data.as_object().cloned().unwrap_or_default();
    map.entry("kind".to_string())
        .or_insert_with(|| Value::String("mission".to_string()));
    map.entry("family".to_string())
        .or_insert_with(|| Value::String(family.to_string()));
    map.entry("stage".to_string())
        .or_insert_with(|| Value::String(family.to_string()));
    map.entry("title".to_string())
        .or_insert_with(|| Value::String(format!("Restored {family}")));
    map.entry("detail".to_string())
        .or_insert_with(|| Value::String("Restored from content rows.".to_string()));
    map.entry("status".to_string())
        .or_insert_with(|| Value::String(format!("content_{family}s")));
    map.entry("scenario".to_string())
        .or_insert_with(|| Value::String("restored".to_string()));
    map.entry("accent".to_string())
        .or_insert_with(|| Value::String("#94a3b8".to_string()));
    map.entry("properties".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    Value::Object(map)
}

async fn delete_scope_content(
    tx: &mut Transaction<'_, Postgres>,
    space_id: &str,
    mission_id: &str,
) -> Result<(), String> {
    sqlx::query(
        r#"
        DELETE FROM content_chains
        WHERE editor_data ->> 'spaceId' = $1
          AND ($2 = '' OR editor_data ->> 'missionId' = $2)
        "#,
    )
    .bind(space_id)
    .bind(mission_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("failed to clear existing content chains: {error}"))?;

    sqlx::query(
        r#"
        DELETE FROM content_editor_scopes
        WHERE space_id = $1 AND mission_id = $2
        "#,
    )
    .bind(space_id)
    .bind(mission_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("failed to clear existing editor scope metadata: {error}"))?;

    Ok(())
}

async fn insert_chain(
    tx: &mut Transaction<'_, Postgres>,
    chain: &ParsedChain,
) -> Result<i64, String> {
    sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO content_chains (name, scope_type, scope_id, enabled, priority, description, editor_data)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING chain_id
        "#,
    )
    .bind(&chain.name)
    .bind(&chain.scope_type)
    .bind(chain.scope_id)
    .bind(chain.enabled)
    .bind(chain.priority)
    .bind(&chain.description)
    .bind(&chain.editor_data)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| format!("failed to insert content chain {}: {error}", chain.name))
}

async fn insert_trigger(
    tx: &mut Transaction<'_, Postgres>,
    chain_id: i64,
    trigger: &ParsedTrigger,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order, editor_data)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(chain_id)
    .bind(&trigger.event_type)
    .bind(&trigger.event_key)
    .bind(&trigger.scope)
    .bind(trigger.once)
    .bind(trigger.sort_order)
    .bind(&trigger.editor_data)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("failed to insert trigger {}: {error}", trigger.node_id))
    .map(|_| ())
}

async fn insert_condition(
    tx: &mut Transaction<'_, Postgres>,
    chain_id: i64,
    condition: &ParsedCondition,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order, editor_data)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(chain_id)
    .bind(&condition.condition_type)
    .bind(condition.target_id)
    .bind(&condition.target_key)
    .bind(&condition.operator)
    .bind(&condition.value)
    .bind(condition.sort_order)
    .bind(&condition.editor_data)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("failed to insert condition {}: {error}", condition.node_id))
    .map(|_| ())
}

async fn insert_action(
    tx: &mut Transaction<'_, Postgres>,
    chain_id: i64,
    action: &ParsedAction,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order, editor_data)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(chain_id)
    .bind(&action.action_type)
    .bind(action.target_id)
    .bind(&action.target_key)
    .bind(&action.params)
    .bind(action.delay_ms)
    .bind(action.sort_order)
    .bind(&action.editor_data)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("failed to insert action {}: {error}", action.node_id))
    .map(|_| ())
}

async fn insert_counter(
    tx: &mut Transaction<'_, Postgres>,
    chain_id: i64,
    counter: &ParsedCounter,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on, editor_data)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(chain_id)
    .bind(&counter.counter_name)
    .bind(counter.target_value)
    .bind(&counter.reset_on)
    .bind(&counter.editor_data)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("failed to insert counter {}: {error}", counter.node_id))
    .map(|_| ())
}

async fn ensure_content_schema(pool: &PgPool) -> Result<(), String> {
    for statement in [
        CREATE_CONTENT_CHAINS_TABLE,
        CREATE_CONTENT_CHAINS_SCOPE_INDEX,
        CREATE_CONTENT_TRIGGERS_TABLE,
        CREATE_CONTENT_TRIGGERS_CHAIN_INDEX,
        CREATE_CONTENT_TRIGGERS_EVENT_INDEX,
        CREATE_CONTENT_CONDITIONS_TABLE,
        CREATE_CONTENT_CONDITIONS_CHAIN_INDEX,
        CREATE_CONTENT_ACTIONS_TABLE,
        CREATE_CONTENT_ACTIONS_CHAIN_INDEX,
        CREATE_CONTENT_COUNTERS_TABLE,
        CREATE_CONTENT_COUNTERS_CHAIN_INDEX,
        CREATE_CONTENT_EDITOR_SCOPES_TABLE,
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .map_err(|error| format!("failed to prepare content schema: {error}"))?;
    }

    Ok(())
}

fn normalize_mission_id(mission_id: Option<String>) -> String {
    mission_id.unwrap_or_default()
}

#[derive(Clone, Debug)]
struct ParsedScopePayload {
    original_payload: Value,
    chains: Vec<ParsedChain>,
}

#[derive(Clone, Debug)]
struct ParsedChain {
    name: String,
    scope_type: String,
    scope_id: Option<i32>,
    enabled: bool,
    priority: i32,
    description: Option<String>,
    editor_data: Value,
    triggers: Vec<ParsedTrigger>,
    conditions: Vec<ParsedCondition>,
    actions: Vec<ParsedAction>,
    counters: Vec<ParsedCounter>,
}

#[derive(Clone, Debug)]
struct ParsedTrigger {
    node_id: String,
    event_type: String,
    event_key: String,
    scope: String,
    once: bool,
    sort_order: i32,
    editor_data: Value,
}

#[derive(Clone, Debug)]
struct ParsedCondition {
    node_id: String,
    condition_type: String,
    target_id: Option<i32>,
    target_key: Option<String>,
    operator: String,
    value: String,
    sort_order: i32,
    editor_data: Value,
}

#[derive(Clone, Debug)]
struct ParsedAction {
    node_id: String,
    action_type: String,
    target_id: Option<i32>,
    target_key: Option<String>,
    params: Value,
    delay_ms: i32,
    sort_order: i32,
    editor_data: Value,
}

#[derive(Clone, Debug)]
struct ParsedCounter {
    node_id: String,
    counter_name: String,
    target_value: i32,
    reset_on: String,
    editor_data: Value,
}

#[derive(Clone, Debug)]
struct EditorNode {
    id: String,
    parent_id: Option<String>,
    position: Value,
    data: Value,
}

fn parse_scope_payload(
    payload: Value,
    expected_space_id: &str,
    expected_mission_id: &str,
) -> Result<ParsedScopePayload, String> {
    let Some(object) = payload.as_object() else {
        return Err("chain editor content payload must be a JSON object".to_string());
    };

    let payload_space_id = object
        .get("spaceId")
        .and_then(Value::as_str)
        .ok_or_else(|| "chain editor content payload is missing spaceId".to_string())?;
    if payload_space_id != expected_space_id {
        return Err(format!(
            "payload spaceId {} did not match requested scope {}",
            payload_space_id, expected_space_id
        ));
    }

    let payload_mission_id = object
        .get("missionId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if payload_mission_id != expected_mission_id {
        return Err(format!(
            "payload missionId {} did not match requested scope {}",
            payload_mission_id, expected_mission_id
        ));
    }

    let Some(node_values) = object.get("nodes").and_then(Value::as_array) else {
        return Err("chain editor content payload is missing nodes".to_string());
    };

    let nodes = node_values
        .iter()
        .map(parse_editor_node)
        .collect::<Result<Vec<_>, _>>()?;

    let mut nodes_by_id = HashMap::new();
    for node in nodes {
        nodes_by_id.insert(node.id.clone(), node);
    }

    let mut chains = Vec::new();
    let mut sorted_chain_ids = nodes_by_id
        .values()
        .filter(|node| node_kind(node) == Some("chain"))
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    sorted_chain_ids.sort();

    for chain_id in sorted_chain_ids {
        let chain_node = nodes_by_id
            .get(&chain_id)
            .ok_or_else(|| format!("missing chain node {chain_id}"))?;
        chains.push(parse_chain(
            chain_node,
            &nodes_by_id,
            expected_space_id,
            expected_mission_id,
        )?);
    }

    Ok(ParsedScopePayload {
        original_payload: payload,
        chains,
    })
}

fn parse_editor_node(value: &Value) -> Result<EditorNode, String> {
    let Some(object) = value.as_object() else {
        return Err("editor node must be a JSON object".to_string());
    };

    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "editor node is missing id".to_string())?
        .to_string();

    let position = object
        .get("position")
        .cloned()
        .unwrap_or_else(|| json!({ "x": 0, "y": 0 }));

    Ok(EditorNode {
        id,
        parent_id: object
            .get("parentId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        position,
        data: object
            .get("data")
            .cloned()
            .ok_or_else(|| "editor node is missing data".to_string())?,
    })
}

fn parse_chain(
    chain_node: &EditorNode,
    nodes_by_id: &HashMap<String, EditorNode>,
    expected_space_id: &str,
    expected_mission_id: &str,
) -> Result<ParsedChain, String> {
    let data = chain_node
        .data
        .as_object()
        .ok_or_else(|| format!("chain node {} data must be an object", chain_node.id))?;

    let scope_type = string_field(data, "scopeType").unwrap_or_else(|| "mission".to_string());
    if !matches!(
        scope_type.as_str(),
        "mission" | "space" | "effect" | "global"
    ) {
        return Err(format!(
            "chain node {} has unsupported scopeType {}",
            chain_node.id, scope_type
        ));
    }

    let space_id = string_field(data, "spaceId").unwrap_or_else(|| expected_space_id.to_string());
    if space_id != expected_space_id {
        return Err(format!(
            "chain node {} belongs to space {} instead of {}",
            chain_node.id, space_id, expected_space_id
        ));
    }

    let mission_id_value = optional_string_field(data, "missionId");
    let normalized_mission_id = mission_id_value.as_deref().unwrap_or_default();
    if normalized_mission_id != expected_mission_id {
        return Err(format!(
            "chain node {} belongs to mission {} instead of {}",
            chain_node.id, normalized_mission_id, expected_mission_id
        ));
    }

    let mut triggers = Vec::new();
    let mut conditions = Vec::new();
    let mut actions = Vec::new();
    let mut counters = Vec::new();

    let mut children = nodes_by_id
        .values()
        .filter(|node| node.parent_id.as_deref() == Some(chain_node.id.as_str()))
        .collect::<Vec<_>>();
    children.sort_by_key(|node| {
        let sort_order = node
            .data
            .get("sortOrder")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX);
        let family_rank = family_sort_rank(node);
        (sort_order, family_rank, node.id.clone())
    });

    for (index, child) in children.iter().enumerate() {
        match family_field(child) {
            Some("trigger") => triggers.push(parse_trigger(child, index as i32)?),
            Some("condition") => conditions.push(parse_condition(child, index as i32)?),
            Some("action") => actions.push(parse_action(child, index as i32)?),
            Some("counter") => counters.push(parse_counter(child)?),
            Some("anchor") => {}
            Some(other) => {
                return Err(format!(
                    "child node {} has unsupported family {}",
                    child.id, other
                ))
            }
            None => return Err(format!("child node {} is missing family", child.id)),
        }
    }

    let counts = json!({
        "anchor": children.iter().filter(|child| family_field(child) == Some("anchor")).count(),
        "trigger": triggers.len(),
        "condition": conditions.len(),
        "action": actions.len(),
        "counter": counters.len(),
    });

    Ok(ParsedChain {
        name: string_field(data, "name").unwrap_or_else(|| chain_node.id.clone()),
        scope_type: scope_type.clone(),
        scope_id: parse_optional_i32(
            optional_string_field(data, "scopeId")
                .or_else(|| mission_id_value.clone())
                .as_deref(),
        )?,
        enabled: bool_field(data, "enabled").unwrap_or(true),
        priority: i32_field(data, "priority").unwrap_or_default(),
        description: optional_string_field(data, "summary"),
        editor_data: json!({
            "nodeId": chain_node.id,
            "position": chain_node.position,
            "source": string_field(data, "source").unwrap_or_else(|| "content_chains".to_string()),
            "semantic": string_field(data, "semantic").unwrap_or_else(|| "Story".to_string()),
            "color": string_field(data, "color").unwrap_or_else(|| "#22c55e".to_string()),
            "spaceId": space_id,
            "missionId": if normalized_mission_id.is_empty() { Value::Null } else { Value::String(normalized_mission_id.to_string()) },
            "counts": counts,
            "sequenceCount": data.get("sequenceCount").cloned().unwrap_or_else(|| json!(0)),
        }),
        triggers,
        conditions,
        actions,
        counters,
    })
}

fn parse_trigger(node: &EditorNode, sort_order: i32) -> Result<ParsedTrigger, String> {
    let data = node
        .data
        .as_object()
        .ok_or_else(|| format!("trigger node {} data must be an object", node.id))?;
    let properties = property_map(data);

    Ok(ParsedTrigger {
        node_id: node.id.clone(),
        event_type: property_or_default(&properties, "event_type", "custom_event"),
        event_key: property_or_default(&properties, "event_key", &node.id),
        scope: property_or_default(&properties, "scope", "player"),
        once: property_or_default(&properties, "once", "false") == "true",
        sort_order,
        editor_data: build_child_editor_data(node, data),
    })
}

fn parse_condition(node: &EditorNode, sort_order: i32) -> Result<ParsedCondition, String> {
    let data = node
        .data
        .as_object()
        .ok_or_else(|| format!("condition node {} data must be an object", node.id))?;
    let properties = property_map(data);

    Ok(ParsedCondition {
        node_id: node.id.clone(),
        condition_type: property_or_default(&properties, "condition_type", "custom_condition"),
        target_id: parse_optional_i32(properties.get("target_id").map(String::as_str))?,
        target_key: properties.get("target_key").cloned(),
        operator: property_or_default(&properties, "operator", "eq"),
        value: property_or_default(&properties, "value", "true"),
        sort_order,
        editor_data: build_child_editor_data(node, data),
    })
}

fn parse_action(node: &EditorNode, sort_order: i32) -> Result<ParsedAction, String> {
    let data = node
        .data
        .as_object()
        .ok_or_else(|| format!("action node {} data must be an object", node.id))?;
    let properties = property_map(data);

    Ok(ParsedAction {
        node_id: node.id.clone(),
        action_type: property_or_default(&properties, "action_type", "custom_action"),
        target_id: parse_optional_i32(
            properties
                .get("target_id")
                .or_else(|| properties.get("dialog_id"))
                .map(String::as_str),
        )?,
        target_key: properties
            .get("target_key")
            .cloned()
            .or_else(|| properties.get("sequence_id").cloned()),
        params: json!({
            "properties": data.get("properties").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
            "scenario": data.get("scenario").cloned().unwrap_or_else(|| Value::String(String::new())),
        }),
        delay_ms: parse_i32_or_default(properties.get("delay_ms").map(String::as_str), 0)?,
        sort_order,
        editor_data: build_child_editor_data(node, data),
    })
}

fn parse_counter(node: &EditorNode) -> Result<ParsedCounter, String> {
    let data = node
        .data
        .as_object()
        .ok_or_else(|| format!("counter node {} data must be an object", node.id))?;
    let properties = property_map(data);

    Ok(ParsedCounter {
        node_id: node.id.clone(),
        counter_name: property_or_default(&properties, "counter_name", &node.id),
        target_value: parse_i32_or_default(properties.get("target_value").map(String::as_str), 1)?,
        reset_on: property_or_default(&properties, "reset_on", "mission_complete"),
        editor_data: build_child_editor_data(node, data),
    })
}

fn build_child_editor_data(node: &EditorNode, data: &Map<String, Value>) -> Value {
    let mut map = Map::new();
    map.insert("nodeId".to_string(), Value::String(node.id.clone()));
    map.insert("position".to_string(), node.position.clone());

    for key in [
        "kind",
        "family",
        "stage",
        "title",
        "detail",
        "status",
        "scenario",
        "accent",
        "threadColor",
        "threadName",
        "manualPosition",
        "sortOrder",
        "inputs",
        "outputs",
        "outputConditions",
        "properties",
    ] {
        if let Some(value) = data.get(key) {
            map.insert(key.to_string(), value.clone());
        }
    }

    Value::Object(map)
}

fn property_map(data: &Map<String, Value>) -> HashMap<String, String> {
    data.get("properties")
        .and_then(Value::as_array)
        .map(|properties| {
            properties
                .iter()
                .filter_map(|property| {
                    let object = property.as_object()?;
                    Some((
                        object.get("label")?.as_str()?.to_string(),
                        object.get("value")?.as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn node_kind(node: &EditorNode) -> Option<&str> {
    node.data.get("kind").and_then(Value::as_str)
}

fn family_field(node: &EditorNode) -> Option<&str> {
    node.data.get("family").and_then(Value::as_str)
}

fn family_sort_rank(node: &EditorNode) -> i32 {
    match family_field(node) {
        Some("anchor") => 0,
        Some("trigger") => 1,
        Some("condition") => 2,
        Some("action") => 3,
        Some("counter") => 4,
        _ => 99,
    }
}

fn string_field(data: &Map<String, Value>, key: &str) -> Option<String> {
    data.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn optional_string_field(data: &Map<String, Value>, key: &str) -> Option<String> {
    match data.get(key) {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) | None => None,
        _ => None,
    }
}

fn bool_field(data: &Map<String, Value>, key: &str) -> Option<bool> {
    data.get(key).and_then(Value::as_bool)
}

fn i32_field(data: &Map<String, Value>, key: &str) -> Option<i32> {
    data.get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn property_or_default(properties: &HashMap<String, String>, key: &str, default: &str) -> String {
    properties
        .get(key)
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

fn parse_optional_i32(value: Option<&str>) -> Result<Option<i32>, String> {
    value
        .map(|raw| {
            let digits = raw
                .split(|character: char| !character.is_ascii_digit() && character != '-')
                .find(|part| !part.is_empty())
                .ok_or_else(|| format!("could not parse integer from {}", raw))?;
            digits
                .parse::<i32>()
                .map(Some)
                .map_err(|error| format!("could not parse integer from {}: {}", raw, error))
        })
        .transpose()
}

fn parse_i32_or_default(value: Option<&str>, default: i32) -> Result<i32, String> {
    Ok(parse_optional_i32(value)?.unwrap_or(default))
}

fn sqlx_error(error: sqlx::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::{normalize_mission_id, parse_scope_payload};
    use serde_json::json;

    #[test]
    fn normalize_mission_id_defaults_to_empty_scope() {
        assert_eq!(normalize_mission_id(None), "");
    }

    #[test]
    fn normalize_mission_id_preserves_specific_scope() {
        assert_eq!(normalize_mission_id(Some("638".to_string())), "638");
    }

    #[test]
    fn parse_scope_payload_extracts_runtime_rows() {
        let payload = json!({
            "version": 1,
            "spaceId": "Castle_CellBlock",
            "missionId": "622",
            "nodes": [
                {
                    "id": "chain-1",
                    "position": { "x": 0, "y": 0 },
                    "data": {
                        "kind": "chain",
                        "name": "Arm Yourself - Crate Interact",
                        "summary": "Mission-scoped crate interaction chain.",
                        "scopeType": "mission",
                        "scopeId": "622",
                        "enabled": true,
                        "priority": 110,
                        "source": "content_chains #2-3",
                        "semantic": "Reward",
                        "color": "#eab308",
                        "spaceId": "Castle_CellBlock",
                        "missionId": "622"
                    }
                },
                {
                    "id": "trigger-1",
                    "parentId": "chain-1",
                    "position": { "x": 100, "y": 80 },
                    "data": {
                        "kind": "mission",
                        "family": "trigger",
                        "stage": "Trigger",
                        "title": "Interact Wooden Crate",
                        "detail": "Player interacts with crate.",
                        "status": "content_triggers",
                        "scenario": "mission interaction",
                        "accent": "#13a2a4",
                        "properties": [
                            { "label": "event_type", "value": "interact_tag" },
                            { "label": "event_key", "value": "Cellblock_WoodenCrate" },
                            { "label": "scope", "value": "player" }
                        ]
                    }
                },
                {
                    "id": "condition-1",
                    "parentId": "chain-1",
                    "position": { "x": 220, "y": 80 },
                    "data": {
                        "kind": "mission",
                        "family": "condition",
                        "stage": "Condition",
                        "title": "Step 2114 Active",
                        "detail": "Only continue for active step.",
                        "status": "content_conditions",
                        "scenario": "step gate",
                        "accent": "#f5aa31",
                        "properties": [
                            { "label": "condition_type", "value": "step_status" },
                            { "label": "target_id", "value": "622" },
                            { "label": "target_key", "value": "2114" },
                            { "label": "value", "value": "active" }
                        ]
                    }
                },
                {
                    "id": "action-1",
                    "parentId": "chain-1",
                    "position": { "x": 340, "y": 80 },
                    "data": {
                        "kind": "mission",
                        "family": "action",
                        "stage": "Action",
                        "title": "Advance step",
                        "detail": "Advance mission step.",
                        "status": "content_actions",
                        "scenario": "mission handoff",
                        "accent": "#22c55e",
                        "properties": [
                            { "label": "action_type", "value": "advance_step" },
                            { "label": "target_id", "value": "622" },
                            { "label": "target_key", "value": "2114" }
                        ]
                    }
                }
            ],
            "edges": [],
            "selectedChainId": "chain-1",
            "selectedNodeId": "action-1",
            "selectedSequenceId": "sequence-1"
        });

        let parsed = parse_scope_payload(payload, "Castle_CellBlock", "622").unwrap();
        assert_eq!(parsed.chains.len(), 1);
        let chain = &parsed.chains[0];
        assert_eq!(chain.name, "Arm Yourself - Crate Interact");
        assert_eq!(chain.scope_type, "mission");
        assert_eq!(chain.scope_id, Some(622));
        assert_eq!(chain.triggers.len(), 1);
        assert_eq!(chain.conditions.len(), 1);
        assert_eq!(chain.actions.len(), 1);
        assert_eq!(chain.triggers[0].event_type, "interact_tag");
        assert_eq!(chain.conditions[0].target_key.as_deref(), Some("2114"));
        assert_eq!(chain.actions[0].action_type, "advance_step");
    }

    #[test]
    fn parse_scope_payload_rejects_scope_mismatch() {
        let payload = json!({
            "version": 1,
            "spaceId": "Castle",
            "missionId": null,
            "nodes": [],
            "edges": []
        });

        let error = parse_scope_payload(payload, "Castle_CellBlock", "").unwrap_err();
        assert!(error.contains("payload spaceId"));
    }
}

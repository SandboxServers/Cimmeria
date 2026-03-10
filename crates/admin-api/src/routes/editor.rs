//! Chain editor persistence endpoints.
//!
//! Provides REST endpoints for loading, saving, and deleting content chains
//! used by the visual chain editor in the ContentEditor desktop app.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use utoipa::ToSchema;

use cimmeria_services::orchestrator::Orchestrator;

// ── Response types ──

#[derive(Serialize)]
pub struct ChainRow {
    pub chain_id: i32,
    pub description: Option<String>,
    pub scope_type: String,
    pub scope_id: Option<i32>,
    pub enabled: bool,
    pub priority: i32,
    pub triggers: Vec<TriggerRow>,
    pub conditions: Vec<ConditionRow>,
    pub actions: Vec<ActionRow>,
    pub counters: Vec<CounterRow>,
}

#[derive(Serialize)]
pub struct TriggerRow {
    pub trigger_id: i32,
    pub event_type: String,
    pub event_key: Option<String>,
    pub scope: String,
    pub once: bool,
    pub sort_order: i32,
}

#[derive(Serialize)]
pub struct ConditionRow {
    pub condition_id: i32,
    pub condition_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub operator: String,
    pub value: Option<String>,
    pub sort_order: i32,
}

#[derive(Serialize)]
pub struct ActionRow {
    pub action_id: i32,
    pub action_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub params: Value,
    pub delay_ms: i32,
    pub sort_order: i32,
}

#[derive(Serialize)]
pub struct CounterRow {
    pub counter_id: i32,
    pub counter_name: String,
    pub target_value: i32,
    pub reset_on: String,
}

// ── Save request types ──

#[derive(Deserialize, ToSchema)]
pub struct SaveRequest {
    pub chains: Vec<SaveChainInput>,
}

#[derive(Deserialize, ToSchema)]
pub struct SaveChainInput {
    pub chain_id: Option<i32>,
    pub description: Option<String>,
    pub scope_type: String,
    pub scope_id: Option<i32>,
    pub enabled: bool,
    pub priority: i32,
    pub triggers: Vec<SaveTriggerInput>,
    pub conditions: Vec<SaveConditionInput>,
    pub actions: Vec<SaveActionInput>,
    pub counters: Vec<SaveCounterInput>,
}

#[derive(Deserialize, ToSchema)]
pub struct SaveTriggerInput {
    pub event_type: String,
    pub event_key: Option<String>,
    pub scope: String,
    pub once: bool,
    pub sort_order: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct SaveConditionInput {
    pub condition_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub operator: String,
    pub value: Option<String>,
    pub sort_order: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct SaveActionInput {
    pub action_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub params: Value,
    pub delay_ms: i32,
    pub sort_order: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct SaveCounterInput {
    pub counter_name: String,
    pub target_value: i32,
    pub reset_on: String,
}

// ── Routes ──

/// Build editor routes.
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/content/{id}", get(load_content).delete(delete_content))
        .route("/content/{scope_id}/{mission_id}", get(load_content_with_mission).delete(delete_content_with_mission))
        .route("/content", post(save_content))
        .route("/draft/{scope_id}", get(load_draft))
        .route("/draft/{scope_id}/{mission_id}", get(load_draft_with_mission))
        .route("/draft", post(save_draft))
}

// ── Helpers ──

async fn fetch_chains(
    pool: &sqlx::PgPool,
    scope_type: &str,
    scope_id: Option<i32>,
) -> Result<Vec<ChainRow>, String> {
    let chain_rows = sqlx::query(
        r#"
        SELECT chain_id, description, scope_type, scope_id, enabled, priority
        FROM content_chains
        WHERE scope_type = $1
          AND (($2::int IS NULL AND scope_id IS NULL) OR scope_id = $2)
        ORDER BY priority DESC, chain_id ASC
        "#,
    )
    .bind(scope_type)
    .bind(scope_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load chains: {e}"))?;

    let mut chains = Vec::new();

    for row in chain_rows {
        let chain_id: i32 = row.get("chain_id");

        let triggers = sqlx::query(
            "SELECT trigger_id, event_type, event_key, scope, once, sort_order \
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
        })
        .collect();

        let conditions = sqlx::query(
            "SELECT condition_id, condition_type, target_id, target_key, operator, value, sort_order \
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
        })
        .collect();

        let actions = sqlx::query(
            "SELECT action_id, action_type, target_id, target_key, params, delay_ms, sort_order \
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
        })
        .collect();

        let counters = sqlx::query(
            "SELECT counter_id, counter_name, target_value, reset_on \
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
        })
        .collect();

        chains.push(ChainRow {
            chain_id,
            description: row.get("description"),
            scope_type: row.get("scope_type"),
            scope_id: row.get("scope_id"),
            enabled: row.get("enabled"),
            priority: row.get("priority"),
            triggers,
            conditions,
            actions,
            counters,
        });
    }

    Ok(chains)
}

fn parse_scope(scope_id: &str) -> (&str, Option<i32>) {
    let parts: Vec<&str> = scope_id.splitn(2, ':').collect();
    let scope_type = parts.first().copied().unwrap_or("space");
    let id = parts.get(1).and_then(|s| s.parse().ok());
    (scope_type, id)
}

// ── Handlers ──

/// Load all chains for a given scope.
#[utoipa::path(
    get,
    path = "/api/editor/content/{id}",
    params(("id" = String, Path, description = "Scope identifier (e.g. space:1)")),
    responses((status = 200, description = "Chain list", body = serde_json::Value)),
    tag = "Editor"
)]
pub async fn load_content(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let pool = {
        let state = orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|db| db.pool().clone())
    };

    let Some(pool) = pool else {
        return Json(serde_json::json!({ "error": "Database unavailable" }));
    };

    let (scope_type, scope_id) = parse_scope(&id);

    match fetch_chains(&pool, scope_type, scope_id).await {
        Ok(chains) => Json(serde_json::json!({ "status": "ok", "chains": chains })),
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}

/// Load chains for a scope filtered by mission.
#[utoipa::path(
    get,
    path = "/api/editor/content/{scope_id}/{mission_id}",
    params(
        ("scope_id" = String, Path, description = "Scope identifier"),
        ("mission_id" = i32, Path, description = "Mission ID filter")
    ),
    responses((status = 200, description = "Chain list", body = serde_json::Value)),
    tag = "Editor"
)]
pub async fn load_content_with_mission(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path((scope_id, _mission_id)): Path<(String, i32)>,
) -> Json<serde_json::Value> {
    // For now, same as load_content — mission filtering is TODO
    let pool = {
        let state = orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|db| db.pool().clone())
    };

    let Some(pool) = pool else {
        return Json(serde_json::json!({ "error": "Database unavailable" }));
    };

    let (scope_type, id) = parse_scope(&scope_id);

    match fetch_chains(&pool, scope_type, id).await {
        Ok(chains) => Json(serde_json::json!({ "status": "ok", "chains": chains })),
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}

/// Save chains (insert or update).
#[utoipa::path(
    post,
    path = "/api/editor/content",
    request_body = SaveRequest,
    responses((status = 200, description = "Saved chain IDs", body = serde_json::Value)),
    tag = "Editor"
)]
pub async fn save_content(
    State(orchestrator): State<Arc<Orchestrator>>,
    Json(payload): Json<SaveRequest>,
) -> Json<serde_json::Value> {
    let pool = {
        let state = orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|db| db.pool().clone())
    };

    let Some(pool) = pool else {
        return Json(serde_json::json!({ "error": "Database unavailable" }));
    };

    let tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Json(serde_json::json!({ "error": format!("Transaction failed: {e}") })),
    };
    let mut tx = tx;
    let mut saved_ids = Vec::new();

    for chain in &payload.chains {
        // Delete existing chain and children if updating
        if let Some(existing_id) = chain.chain_id {
            for table in &["content_counters", "content_actions", "content_conditions", "content_triggers", "content_chains"] {
                let q = format!("DELETE FROM {table} WHERE chain_id = $1");
                if let Err(e) = sqlx::query(&q).bind(existing_id).execute(&mut *tx).await {
                    return Json(serde_json::json!({ "error": format!("Delete from {table} failed: {e}") }));
                }
            }
        }

        // Insert chain
        let chain_id_result = if let Some(existing_id) = chain.chain_id {
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) \
                 VALUES ($1, $2, $3, $4, $5, $6) RETURNING chain_id",
            )
            .bind(existing_id)
            .bind(&chain.description)
            .bind(&chain.scope_type)
            .bind(chain.scope_id)
            .bind(chain.enabled)
            .bind(chain.priority)
            .fetch_one(&mut *tx)
            .await
        } else {
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO content_chains (description, scope_type, scope_id, enabled, priority) \
                 VALUES ($1, $2, $3, $4, $5) RETURNING chain_id",
            )
            .bind(&chain.description)
            .bind(&chain.scope_type)
            .bind(chain.scope_id)
            .bind(chain.enabled)
            .bind(chain.priority)
            .fetch_one(&mut *tx)
            .await
        };

        let chain_id = match chain_id_result {
            Ok(id) => id,
            Err(e) => return Json(serde_json::json!({ "error": format!("Insert chain failed: {e}") })),
        };

        for t in &chain.triggers {
            if let Err(e) = sqlx::query(
                "INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(chain_id).bind(&t.event_type).bind(&t.event_key)
            .bind(&t.scope).bind(t.once).bind(t.sort_order)
            .execute(&mut *tx).await {
                return Json(serde_json::json!({ "error": format!("Insert trigger failed: {e}") }));
            }
        }

        for c in &chain.conditions {
            if let Err(e) = sqlx::query(
                "INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(chain_id).bind(&c.condition_type).bind(c.target_id).bind(&c.target_key)
            .bind(&c.operator).bind(&c.value).bind(c.sort_order)
            .execute(&mut *tx).await {
                return Json(serde_json::json!({ "error": format!("Insert condition failed: {e}") }));
            }
        }

        for a in &chain.actions {
            if let Err(e) = sqlx::query(
                "INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(chain_id).bind(&a.action_type).bind(a.target_id).bind(&a.target_key)
            .bind(&a.params).bind(a.delay_ms).bind(a.sort_order)
            .execute(&mut *tx).await {
                return Json(serde_json::json!({ "error": format!("Insert action failed: {e}") }));
            }
        }

        for ct in &chain.counters {
            if let Err(e) = sqlx::query(
                "INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(chain_id).bind(&ct.counter_name).bind(ct.target_value).bind(&ct.reset_on)
            .execute(&mut *tx).await {
                return Json(serde_json::json!({ "error": format!("Insert counter failed: {e}") }));
            }
        }

        saved_ids.push(chain_id);
    }

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({ "error": format!("Commit failed: {e}") }));
    }

    Json(serde_json::json!({ "status": "ok", "saved_ids": saved_ids }))
}

/// Delete a single chain by ID.
#[utoipa::path(
    delete,
    path = "/api/editor/content/{id}",
    params(("id" = i32, Path, description = "Chain ID to delete")),
    responses((status = 200, description = "Deletion result", body = serde_json::Value)),
    tag = "Editor"
)]
pub async fn delete_content(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<i32>,
) -> Json<serde_json::Value> {
    let pool = {
        let state = orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|db| db.pool().clone())
    };

    let Some(pool) = pool else {
        return Json(serde_json::json!({ "error": "Database unavailable" }));
    };

    for table in &["content_counters", "content_actions", "content_conditions", "content_triggers", "content_chains"] {
        let q = format!("DELETE FROM {table} WHERE chain_id = $1");
        if let Err(e) = sqlx::query(&q).bind(id).execute(&pool).await {
            return Json(serde_json::json!({ "error": format!("Delete from {table} failed: {e}") }));
        }
    }

    Json(serde_json::json!({ "status": "ok", "deleted": id }))
}

/// Delete chains filtered by scope and mission.
#[utoipa::path(
    delete,
    path = "/api/editor/content/{scope_id}/{mission_id}",
    params(
        ("scope_id" = String, Path, description = "Scope identifier"),
        ("mission_id" = i32, Path, description = "Mission ID")
    ),
    responses((status = 200, description = "Deletion result", body = serde_json::Value)),
    tag = "Editor"
)]
pub async fn delete_content_with_mission(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path((_scope_id, _mission_id)): Path<(String, i32)>,
) -> Json<serde_json::Value> {
    // TODO: implement mission-scoped deletion
    Json(serde_json::json!({ "status": "ok", "message": "not implemented" }))
}

/// Load a draft for a scope (stub).
#[utoipa::path(
    get,
    path = "/api/editor/draft/{scope_id}",
    params(("scope_id" = String, Path, description = "Scope identifier")),
    responses((status = 200, description = "Draft chain list", body = serde_json::Value)),
    tag = "Editor"
)]
pub async fn load_draft(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Path(_scope_id): Path<String>,
) -> Json<serde_json::Value> {
    // TODO: implement draft storage
    Json(serde_json::json!({ "status": "ok", "chains": [] }))
}

/// Load a draft filtered by mission (stub).
#[utoipa::path(
    get,
    path = "/api/editor/draft/{scope_id}/{mission_id}",
    params(
        ("scope_id" = String, Path, description = "Scope identifier"),
        ("mission_id" = i32, Path, description = "Mission ID filter")
    ),
    responses((status = 200, description = "Draft chain list", body = serde_json::Value)),
    tag = "Editor"
)]
pub async fn load_draft_with_mission(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Path((_scope_id, _mission_id)): Path<(String, i32)>,
) -> Json<serde_json::Value> {
    // TODO: implement draft storage
    Json(serde_json::json!({ "status": "ok", "chains": [] }))
}

/// Save a draft (stub).
#[utoipa::path(
    post,
    path = "/api/editor/draft",
    request_body = SaveRequest,
    responses((status = 200, description = "Draft saved", body = serde_json::Value)),
    tag = "Editor"
)]
pub async fn save_draft(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Json(_payload): Json<SaveRequest>,
) -> Json<serde_json::Value> {
    // TODO: implement draft storage
    Json(serde_json::json!({ "status": "ok", "message": "not implemented" }))
}

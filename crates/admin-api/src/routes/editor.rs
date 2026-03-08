//! Chain editor content and draft persistence endpoints.
//!
//! Provides REST endpoints for saving and loading chain editor graph payloads
//! (content scopes) and in-progress drafts. These replace the Tauri `invoke()`
//! IPC so the browser-based frontend can persist editor state to PostgreSQL.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

use cimmeria_services::orchestrator::Orchestrator;

// ---------------------------------------------------------------------------
// Schema DDL
// ---------------------------------------------------------------------------

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

const CREATE_CHAIN_EDITOR_DRAFTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS chain_editor_drafts (
    id BIGSERIAL PRIMARY KEY,
    space_id TEXT NOT NULL,
    mission_id TEXT NOT NULL DEFAULT '',
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chain_editor_drafts_unique_scope UNIQUE (space_id, mission_id)
)
"#;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Request body for saving editor content or drafts.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequest {
    pub space_id: String,
    pub mission_id: Option<String>,
    pub payload: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

/// Build editor routes (nested under `/api/content/editor`).
///
/// - `GET  /content/:space_id`              - load content (no mission filter)
/// - `GET  /content/:space_id/:mission_id`  - load content for space+mission
/// - `POST /content`                        - save content
/// - `GET  /draft/:space_id`                - load draft (no mission filter)
/// - `GET  /draft/:space_id/:mission_id`    - load draft for space+mission
/// - `POST /draft`                          - save draft
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/content/{space_id}", get(load_content))
        .route("/content/{space_id}/{mission_id}", get(load_content_with_mission))
        .route("/content", post(save_content))
        .route("/draft/{space_id}", get(load_draft))
        .route("/draft/{space_id}/{mission_id}", get(load_draft_with_mission))
        .route("/draft", post(save_draft))
}

// ---------------------------------------------------------------------------
// Content handlers
// ---------------------------------------------------------------------------

/// Load editor content for a space.
#[utoipa::path(
    get,
    path = "/api/content/editor/content/{space_id}",
    params(
        ("space_id" = String, Path, description = "Space identifier")
    ),
    responses(
        (status = 200, description = "Editor content payload", body = serde_json::Value)
    ),
    tag = "Editor"
)]
pub async fn load_content(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path(space_id): Path<String>,
) -> Json<serde_json::Value> {
    load_from_table(&orchestrator, "content_editor_scopes", &space_id, "").await
}

/// Load editor content for a space and mission.
#[utoipa::path(
    get,
    path = "/api/content/editor/content/{space_id}/{mission_id}",
    params(
        ("space_id" = String, Path, description = "Space identifier"),
        ("mission_id" = String, Path, description = "Mission identifier")
    ),
    responses(
        (status = 200, description = "Editor content payload", body = serde_json::Value)
    ),
    tag = "Editor"
)]
pub async fn load_content_with_mission(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path((space_id, mission_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    load_from_table(&orchestrator, "content_editor_scopes", &space_id, &mission_id).await
}

/// Save editor content.
#[utoipa::path(
    post,
    path = "/api/content/editor/content",
    request_body = SaveRequest,
    responses(
        (status = 200, description = "Save result", body = serde_json::Value)
    ),
    tag = "Editor"
)]
pub async fn save_content(
    State(orchestrator): State<Arc<Orchestrator>>,
    Json(body): Json<SaveRequest>,
) -> Json<serde_json::Value> {
    save_to_table(&orchestrator, "content_editor_scopes", body).await
}

// ---------------------------------------------------------------------------
// Draft handlers
// ---------------------------------------------------------------------------

/// Load editor draft for a space.
#[utoipa::path(
    get,
    path = "/api/content/editor/draft/{space_id}",
    params(
        ("space_id" = String, Path, description = "Space identifier")
    ),
    responses(
        (status = 200, description = "Draft payload", body = serde_json::Value)
    ),
    tag = "Editor"
)]
pub async fn load_draft(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path(space_id): Path<String>,
) -> Json<serde_json::Value> {
    load_from_table(&orchestrator, "chain_editor_drafts", &space_id, "").await
}

/// Load editor draft for a space and mission.
#[utoipa::path(
    get,
    path = "/api/content/editor/draft/{space_id}/{mission_id}",
    params(
        ("space_id" = String, Path, description = "Space identifier"),
        ("mission_id" = String, Path, description = "Mission identifier")
    ),
    responses(
        (status = 200, description = "Draft payload", body = serde_json::Value)
    ),
    tag = "Editor"
)]
pub async fn load_draft_with_mission(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path((space_id, mission_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    load_from_table(&orchestrator, "chain_editor_drafts", &space_id, &mission_id).await
}

/// Save editor draft.
#[utoipa::path(
    post,
    path = "/api/content/editor/draft",
    request_body = SaveRequest,
    responses(
        (status = 200, description = "Save result", body = serde_json::Value)
    ),
    tag = "Editor"
)]
pub async fn save_draft(
    State(orchestrator): State<Arc<Orchestrator>>,
    Json(body): Json<SaveRequest>,
) -> Json<serde_json::Value> {
    save_to_table(&orchestrator, "chain_editor_drafts", body).await
}

// ---------------------------------------------------------------------------
// Shared DB helpers
// ---------------------------------------------------------------------------

/// Acquire the DB pool from the orchestrator state, or return `None`.
async fn get_pool(orchestrator: &Orchestrator) -> Option<PgPool> {
    let state = orchestrator.state();
    let state = state.read().await;
    state.db.as_ref().map(|db| db.pool().clone())
}

/// Normalise `mission_id`: `None` / `null` -> empty string (matches Tauri behaviour).
fn normalize_mission_id(mission_id: Option<String>) -> String {
    mission_id.unwrap_or_default()
}

/// Generic loader for both content_editor_scopes and chain_editor_drafts.
///
/// Both tables share the same (space_id, mission_id) -> payload shape, so one
/// function serves both with only the table name varying.
async fn load_from_table(
    orchestrator: &Orchestrator,
    table: &str,
    space_id: &str,
    mission_id: &str,
) -> Json<serde_json::Value> {
    let Some(pool) = get_pool(orchestrator).await else {
        return Json(serde_json::json!({
            "status": "ok",
            "available": false,
            "reason": "Database unavailable.",
            "payload": null
        }));
    };

    if let Err(error) = ensure_table(&pool, table).await {
        return Json(serde_json::json!({
            "status": "error",
            "available": false,
            "reason": error,
            "payload": null
        }));
    }

    // Both tables use the same column names; only the table name differs.
    let query = format!(
        "SELECT payload FROM {table} WHERE space_id = $1 AND mission_id = $2"
    );

    match sqlx::query_scalar::<_, serde_json::Value>(&query)
        .bind(space_id)
        .bind(mission_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(payload) => Json(serde_json::json!({
            "status": "ok",
            "available": true,
            "payload": payload
        })),
        Err(error) => Json(serde_json::json!({
            "status": "error",
            "available": true,
            "reason": error.to_string(),
            "payload": null
        })),
    }
}

/// Generic saver for both content_editor_scopes and chain_editor_drafts.
async fn save_to_table(
    orchestrator: &Orchestrator,
    table: &str,
    body: SaveRequest,
) -> Json<serde_json::Value> {
    let Some(pool) = get_pool(orchestrator).await else {
        return Json(serde_json::json!({
            "status": "error",
            "saved": false,
            "reason": "Database unavailable."
        }));
    };

    if let Err(error) = ensure_table(&pool, table).await {
        return Json(serde_json::json!({
            "status": "error",
            "saved": false,
            "reason": error
        }));
    }

    let mission_id = normalize_mission_id(body.mission_id);

    let query = format!(
        r#"
        INSERT INTO {table} (space_id, mission_id, payload)
        VALUES ($1, $2, $3)
        ON CONFLICT (space_id, mission_id)
        DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()
        "#
    );

    match sqlx::query(&query)
        .bind(&body.space_id)
        .bind(&mission_id)
        .bind(&body.payload)
        .execute(&pool)
        .await
    {
        Ok(_) => Json(serde_json::json!({
            "status": "ok",
            "saved": true
        })),
        Err(error) => Json(serde_json::json!({
            "status": "error",
            "saved": false,
            "reason": error.to_string()
        })),
    }
}

/// Run CREATE TABLE IF NOT EXISTS for the requested table.
async fn ensure_table(pool: &PgPool, table: &str) -> Result<(), String> {
    let ddl = match table {
        "content_editor_scopes" => CREATE_CONTENT_EDITOR_SCOPES_TABLE,
        "chain_editor_drafts" => CREATE_CHAIN_EDITOR_DRAFTS_TABLE,
        other => return Err(format!("unknown editor table: {other}")),
    };

    sqlx::query(ddl)
        .execute(pool)
        .await
        .map_err(|error| format!("failed to ensure {table} schema: {error}"))?;

    Ok(())
}

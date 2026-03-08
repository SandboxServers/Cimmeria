//! Space management endpoints.
//!
//! Provides REST endpoints for listing, inspecting, and managing game spaces
//! (world zones/instances). Used by the ServerEd admin panel for world
//! management and debugging.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::FromRow;

use cimmeria_services::orchestrator::Orchestrator;

#[derive(Serialize, FromRow)]
struct SpaceRecord {
    world_id: i32,
    world: String,
    client_map: String,
    has_script: bool,
    flags: i32,
    mission_count: i64,
}

#[derive(Serialize)]
struct SpaceSummary {
    total_spaces: usize,
    scripted_spaces: usize,
    mission_links: i64,
}

#[derive(Serialize)]
struct SpaceListResponse {
    status: &'static str,
    available: bool,
    reason: Option<String>,
    spaces: Vec<SpaceRecord>,
    summary: SpaceSummary,
}

/// Build space routes.
///
/// - `GET /` - List all active spaces
/// - `GET /:id` - Get space details by ID
/// - `POST /` - Create a new space instance
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/", get(list_spaces).post(create_space))
        .route("/{id}", get(get_space))
}

/// List all active game spaces.
async fn list_spaces(
    State(orchestrator): State<Arc<Orchestrator>>,
) -> Json<SpaceListResponse> {
    let pool = {
        let state = orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|db| db.pool().clone())
    };

    let Some(pool) = pool else {
        return Json(SpaceListResponse {
            status: "ok",
            available: false,
            reason: Some("Database unavailable.".to_string()),
            spaces: Vec::new(),
            summary: SpaceSummary {
                total_spaces: 0,
                scripted_spaces: 0,
                mission_links: 0,
            },
        });
    };

    match sqlx::query_as::<_, SpaceRecord>(
        r#"
        SELECT
            w.world_id,
            w.world,
            w.client_map,
            w.has_script,
            w.flags,
            COUNT(m.mission_id)::bigint AS mission_count
        FROM resources.worlds w
        LEFT JOIN resources.missions m
            ON m.script_spaces = w.world
        GROUP BY w.world_id, w.world, w.client_map, w.has_script, w.flags
        ORDER BY w.world ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(spaces) => {
            let summary = SpaceSummary {
                total_spaces: spaces.len(),
                scripted_spaces: spaces.iter().filter(|space| space.has_script).count(),
                mission_links: spaces.iter().map(|space| space.mission_count).sum(),
            };

            Json(SpaceListResponse {
                status: "ok",
                available: true,
                reason: None,
                spaces,
                summary,
            })
        }
        Err(error) => Json(SpaceListResponse {
            status: "ok",
            available: false,
            reason: Some(error.to_string()),
            spaces: Vec::new(),
            summary: SpaceSummary {
                total_spaces: 0,
                scripted_spaces: 0,
                mission_links: 0,
            },
        }),
    }
}

/// Get detailed information about a specific space.
async fn get_space(
    State(orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<i32>,
) -> Json<serde_json::Value> {
    let pool = {
        let state = orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|db| db.pool().clone())
    };

    let Some(pool) = pool else {
        return Json(serde_json::json!({
            "status": "ok",
            "available": false,
            "reason": "Database unavailable.",
            "space": null,
            "space_id": id
        }));
    };

    let space = sqlx::query_as::<_, SpaceRecord>(
        r#"
        SELECT
            w.world_id,
            w.world,
            w.client_map,
            w.has_script,
            w.flags,
            COUNT(m.mission_id)::bigint AS mission_count
        FROM resources.worlds w
        LEFT JOIN resources.missions m
            ON m.script_spaces = w.world
        WHERE w.world_id = $1
        GROUP BY w.world_id, w.world, w.client_map, w.has_script, w.flags
        "#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;

    match space {
        Ok(space) => Json(serde_json::json!({
            "status": "ok",
            "available": true,
            "space_id": id,
            "space": space,
        })),
        Err(error) => Json(serde_json::json!({
            "status": "ok",
            "available": false,
            "reason": error.to_string(),
            "space_id": id,
            "space": null
        })),
    }
}

/// Create a new space instance.
async fn create_space(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Parse space definition from body, delegate to CellService
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented"
    }))
}

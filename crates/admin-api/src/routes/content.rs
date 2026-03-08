//! Content engine browsing endpoints.
//!
//! Provides REST endpoints for browsing the data-driven content pipeline:
//! items, abilities, missions, loot tables, and other game definitions loaded
//! from the PostgreSQL database.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::FromRow;

use cimmeria_services::orchestrator::Orchestrator;

#[derive(Serialize)]
struct ContentSummary {
    world_count: i64,
    scripted_world_count: i64,
    mission_count: i64,
    story_mission_count: i64,
    hidden_mission_count: i64,
    scripted_mission_count: i64,
}

#[derive(Serialize, FromRow)]
struct SpaceMissionCount {
    scope: String,
    mission_count: i64,
}

#[derive(Serialize)]
struct ContentSummaryResponse {
    status: &'static str,
    available: bool,
    reason: Option<String>,
    summary: ContentSummary,
    top_space_mission_counts: Vec<SpaceMissionCount>,
}

/// Build content routes.
///
/// - `GET /` - List content categories
/// - `GET /items` - List all item definitions
/// - `GET /items/:id` - Get a specific item definition
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/", get(list_categories))
        .route("/summary", get(get_summary))
        .route("/items", get(list_items))
        .route("/items/{id}", get(get_item))
}

/// List available content categories.
async fn list_categories(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "categories": [
            "items", "abilities", "missions", "loot_tables",
            "effects", "dialogs", "entity_templates"
        ]
    }))
}

/// List all item definitions from the content engine.
async fn list_items(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Query content engine for item definitions
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "items": []
    }))
}

/// Get high-level counts for the content browser/dashboard.
async fn get_summary(
    State(orchestrator): State<Arc<Orchestrator>>,
) -> Json<ContentSummaryResponse> {
    let pool = {
        let state = orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|db| db.pool().clone())
    };

    let default_summary = ContentSummary {
        world_count: 0,
        scripted_world_count: 0,
        mission_count: 0,
        story_mission_count: 0,
        hidden_mission_count: 0,
        scripted_mission_count: 0,
    };

    let Some(pool) = pool else {
        return Json(ContentSummaryResponse {
            status: "ok",
            available: false,
            reason: Some("Database unavailable.".to_string()),
            summary: default_summary,
            top_space_mission_counts: Vec::new(),
        });
    };

    let summary_result = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
        r#"
        SELECT
            (SELECT COUNT(*)::bigint FROM resources.worlds) AS world_count,
            (SELECT COUNT(*)::bigint FROM resources.worlds WHERE has_script) AS scripted_world_count,
            (SELECT COUNT(*)::bigint FROM resources.missions) AS mission_count,
            (SELECT COUNT(*)::bigint FROM resources.missions WHERE is_a_story) AS story_mission_count,
            (SELECT COUNT(*)::bigint FROM resources.missions WHERE is_hidden) AS hidden_mission_count,
            (
                SELECT COUNT(*)::bigint
                FROM resources.missions
                WHERE script_spaces IS NOT NULL AND script_spaces <> ''
            ) AS scripted_mission_count
        "#,
    )
    .fetch_one(&pool)
    .await;

    let scopes_result = sqlx::query_as::<_, SpaceMissionCount>(
        r#"
        SELECT
            COALESCE(NULLIF(script_spaces, ''), 'Unassigned') AS scope,
            COUNT(*)::bigint AS mission_count
        FROM resources.missions
        GROUP BY COALESCE(NULLIF(script_spaces, ''), 'Unassigned')
        ORDER BY mission_count DESC, scope ASC
        LIMIT 6
        "#,
    )
    .fetch_all(&pool)
    .await;

    match (summary_result, scopes_result) {
        (Ok((world_count, scripted_world_count, mission_count, story_mission_count, hidden_mission_count, scripted_mission_count)), Ok(top_space_mission_counts)) => Json(ContentSummaryResponse {
            status: "ok",
            available: true,
            reason: None,
            summary: ContentSummary {
                world_count,
                scripted_world_count,
                mission_count,
                story_mission_count,
                hidden_mission_count,
                scripted_mission_count,
            },
            top_space_mission_counts,
        }),
        (Err(error), _) | (_, Err(error)) => Json(ContentSummaryResponse {
            status: "ok",
            available: false,
            reason: Some(error.to_string()),
            summary: default_summary,
            top_space_mission_counts: Vec::new(),
        }),
    }
}

/// Get a specific item definition by ID.
async fn get_item(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<i32>,
) -> Json<serde_json::Value> {
    // TODO: Look up item by ID from the content engine
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "item_id": id
    }))
}

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
use utoipa::ToSchema;

use cimmeria_services::orchestrator::Orchestrator;

#[derive(Serialize, ToSchema)]
pub struct ContentSummary {
    pub world_count: i64,
    pub scripted_world_count: i64,
    pub mission_count: i64,
    pub story_mission_count: i64,
    pub hidden_mission_count: i64,
    pub scripted_mission_count: i64,
}

#[derive(Serialize, FromRow, ToSchema)]
pub struct SpaceMissionCount {
    pub scope: String,
    pub mission_count: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ContentSummaryResponse {
    pub status: &'static str,
    pub available: bool,
    pub reason: Option<String>,
    pub summary: ContentSummary,
    pub top_space_mission_counts: Vec<SpaceMissionCount>,
}

#[derive(Serialize, FromRow, ToSchema)]
pub struct EditorPickerOption {
    pub value: String,
    pub label: String,
}

#[derive(Serialize, FromRow, ToSchema)]
pub struct EditorMissionOption {
    pub value: String,
    pub label: String,
    pub space_id: String,
}

#[derive(Serialize, FromRow, ToSchema)]
pub struct EditorRegionOption {
    pub value: String,
    pub label: String,
    pub space_id: String,
}

#[derive(Serialize, FromRow, ToSchema)]
pub struct EditorStepOption {
    pub value: String,
    pub label: String,
    pub mission_id: String,
}

#[derive(Serialize, ToSchema)]
pub struct EditorPickersResponse {
    pub status: &'static str,
    pub available: bool,
    pub reason: Option<String>,
    pub spaces: Vec<EditorPickerOption>,
    pub missions: Vec<EditorMissionOption>,
    pub dialogs: Vec<EditorPickerOption>,
    pub items: Vec<EditorPickerOption>,
    pub regions: Vec<EditorRegionOption>,
    pub steps: Vec<EditorStepOption>,
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
        .route("/editor-pickers", get(get_editor_pickers))
        .route("/items", get(list_items))
        .route("/items/{id}", get(get_item))
}

/// List available content categories.
#[utoipa::path(
    get,
    path = "/api/content",
    responses(
        (status = 200, description = "Available content categories", body = serde_json::Value)
    ),
    tag = "Content"
)]
pub async fn list_categories(
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
#[utoipa::path(
    get,
    path = "/api/content/items",
    responses(
        (status = 200, description = "Item definition list", body = serde_json::Value)
    ),
    tag = "Content"
)]
pub async fn list_items(
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
#[utoipa::path(
    get,
    path = "/api/content/summary",
    responses(
        (status = 200, description = "Content summary statistics", body = ContentSummaryResponse)
    ),
    tag = "Content"
)]
pub async fn get_summary(
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

/// Get picker options for the chain editor UI.
#[utoipa::path(
    get,
    path = "/api/content/editor-pickers",
    responses(
        (status = 200, description = "Editor picker options for spaces, missions, dialogs, items, regions, and steps", body = EditorPickersResponse)
    ),
    tag = "Content"
)]
pub async fn get_editor_pickers(
    State(orchestrator): State<Arc<Orchestrator>>,
) -> Json<EditorPickersResponse> {
    let pool = {
        let state = orchestrator.state();
        let state = state.read().await;
        state.db.as_ref().map(|db| db.pool().clone())
    };

    let Some(pool) = pool else {
        return Json(EditorPickersResponse {
            status: "ok",
            available: false,
            reason: Some("Database unavailable.".to_string()),
            spaces: Vec::new(),
            missions: Vec::new(),
            dialogs: Vec::new(),
            items: Vec::new(),
            regions: Vec::new(),
            steps: Vec::new(),
        });
    };

    let spaces = sqlx::query_as::<_, EditorPickerOption>(
        r#"
        SELECT
            w.world AS value,
            w.world AS label
        FROM resources.worlds w
        WHERE w.has_script
        ORDER BY w.world ASC
        "#,
    )
    .fetch_all(&pool)
    .await;

    let missions = sqlx::query_as::<_, EditorMissionOption>(
        r#"
        SELECT
            m.mission_id::text AS value,
            m.mission_id::text || ' - ' || m.mission_defn AS label,
            COALESCE(NULLIF(m.script_spaces, ''), '') AS space_id
        FROM resources.missions m
        WHERE m.is_enabled
        ORDER BY m.mission_id ASC
        "#,
    )
    .fetch_all(&pool)
    .await;

    let dialogs = sqlx::query_as::<_, EditorPickerOption>(
        r#"
        SELECT
            d.dialog_id::text AS value,
            d.dialog_id::text || ' - ' || COALESCE(NULLIF(d.name, ''), 'Unnamed Dialog') AS label
        FROM resources.dialogs d
        ORDER BY d.dialog_id ASC
        "#,
    )
    .fetch_all(&pool)
    .await;

    let items = sqlx::query_as::<_, EditorPickerOption>(
        r#"
        SELECT
            i.item_id::text AS value,
            i.item_id::text || ' - ' || i.name AS label
        FROM resources.items i
        ORDER BY i.item_id ASC
        "#,
    )
    .fetch_all(&pool)
    .await;

    let regions = sqlx::query_as::<_, EditorRegionOption>(
        r#"
        SELECT
            w.world || '.' || gr.tag AS value,
            w.world || '.' || gr.tag AS label,
            w.world AS space_id
        FROM resources.generic_regions gr
        INNER JOIN resources.worlds w
            ON w.world_id = gr.world_id
        WHERE gr.tag IS NOT NULL AND gr.tag <> ''
        UNION
        SELECT
            w.world || '.' || rr.tag AS value,
            w.world || '.' || rr.tag AS label,
            w.world AS space_id
        FROM resources.ring_transport_regions rr
        INNER JOIN resources.worlds w
            ON w.world_id = rr.world_id
        WHERE rr.tag IS NOT NULL AND rr.tag <> ''
        ORDER BY label ASC
        "#,
    )
    .fetch_all(&pool)
    .await;

    let steps = sqlx::query_as::<_, EditorStepOption>(
        r#"
        SELECT
            ms.step_id::text AS value,
            ms.step_id::text || ' - ' || ms.step_display_log_text AS label,
            ms.mission_id::text AS mission_id
        FROM resources.mission_steps ms
        ORDER BY ms.mission_id ASC, ms.index ASC NULLS LAST, ms.step_id ASC
        "#,
    )
    .fetch_all(&pool)
    .await;

    match (spaces, missions, dialogs, items, regions, steps) {
        (Ok(spaces), Ok(missions), Ok(dialogs), Ok(items), Ok(regions), Ok(steps)) => {
            Json(EditorPickersResponse {
                status: "ok",
                available: true,
                reason: None,
                spaces,
                missions,
                dialogs,
                items,
                regions,
                steps,
            })
        }
        (Err(error), _, _, _, _, _)
        | (_, Err(error), _, _, _, _)
        | (_, _, Err(error), _, _, _)
        | (_, _, _, Err(error), _, _)
        | (_, _, _, _, Err(error), _)
        | (_, _, _, _, _, Err(error)) => Json(EditorPickersResponse {
            status: "ok",
            available: false,
            reason: Some(error.to_string()),
            spaces: Vec::new(),
            missions: Vec::new(),
            dialogs: Vec::new(),
            items: Vec::new(),
            regions: Vec::new(),
            steps: Vec::new(),
        }),
    }
}

/// Get a specific item definition by ID.
#[utoipa::path(
    get,
    path = "/api/content/items/{id}",
    params(
        ("id" = i32, Path, description = "Item definition ID")
    ),
    responses(
        (status = 200, description = "Item definition details", body = serde_json::Value)
    ),
    tag = "Content"
)]
pub async fn get_item(
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

//! Content engine browsing endpoints.
//!
//! Provides REST endpoints for browsing the data-driven content pipeline:
//! items, abilities, missions, loot tables, and other game definitions loaded
//! from the PostgreSQL database.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use cimmeria_services::orchestrator::Orchestrator;

/// Build content routes.
///
/// - `GET /` - List content categories
/// - `GET /items` - List all item definitions
/// - `GET /items/:id` - Get a specific item definition
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/", get(list_categories))
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

//! Entity inspection and manipulation endpoints.
//!
//! Provides REST endpoints for listing, inspecting, and modifying entities
//! at runtime. Used by the ServerEd admin panel for live entity debugging.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use cimmeria_services::orchestrator::Orchestrator;

/// Build entity routes.
///
/// - `GET /` - List all active entities
/// - `GET /:id` - Get entity details by ID
/// - `POST /:id/property` - Set a property on an entity
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/", get(list_entities))
        .route("/{id}", get(get_entity))
        .route("/{id}/property", post(set_entity_property))
}

/// List all active entities across all services.
async fn list_entities(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Query entity manager for all active entities
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "entities": []
    }))
}

/// Get detailed information about a specific entity.
async fn get_entity(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<i32>,
) -> Json<serde_json::Value> {
    // TODO: Look up entity by ID from the entity manager
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "entity_id": id
    }))
}

/// Set a property value on a specific entity.
async fn set_entity_property(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<i32>,
) -> Json<serde_json::Value> {
    // TODO: Parse property name/value from body, apply to entity
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "entity_id": id
    }))
}

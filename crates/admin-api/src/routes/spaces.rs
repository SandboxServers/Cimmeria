//! Space management endpoints.
//!
//! Provides REST endpoints for listing, inspecting, and managing game spaces
//! (world zones/instances). Used by the ServerEd admin panel for world
//! management and debugging.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use cimmeria_services::orchestrator::Orchestrator;

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
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Query CellService for active spaces
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "spaces": []
    }))
}

/// Get detailed information about a specific space.
async fn get_space(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<i32>,
) -> Json<serde_json::Value> {
    // TODO: Look up space by ID from the cell service
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "space_id": id
    }))
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

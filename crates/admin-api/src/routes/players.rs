//! Player administration endpoints.
//!
//! Provides REST endpoints for listing, inspecting, and administering online
//! players. Includes account management, character inspection, and GM actions
//! like kick, ban, and teleport.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use cimmeria_services::orchestrator::Orchestrator;

/// Build player routes.
///
/// - `GET /` - List online players
/// - `GET /:id` - Get player details
/// - `POST /:id/kick` - Kick a player from the server
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/", get(list_players))
        .route("/{id}", get(get_player))
        .route("/{id}/kick", post(kick_player))
}

/// List all currently online players.
async fn list_players(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Query base service for connected players
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "players": []
    }))
}

/// Get detailed information about a specific player.
async fn get_player(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<i32>,
) -> Json<serde_json::Value> {
    // TODO: Look up player entity and account info
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "player_id": id
    }))
}

/// Kick a player from the server.
async fn kick_player(
    State(_orchestrator): State<Arc<Orchestrator>>,
    Path(id): Path<i32>,
) -> Json<serde_json::Value> {
    // TODO: Send disconnect to player session, clean up entity
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "player_id": id,
        "action": "kick"
    }))
}

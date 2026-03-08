//! Player administration endpoints.
//!
//! Provides REST endpoints for listing, inspecting, and administering online
//! players. Includes account management, character inspection, and GM actions
//! like kick, ban, and teleport.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

use cimmeria_services::orchestrator::Orchestrator;

#[derive(Serialize)]
struct PlayerSummary {
    online_count: usize,
    ready: bool,
}

#[derive(Serialize)]
struct PlayerServiceState {
    auth: bool,
    base: bool,
    cell: bool,
    database: bool,
}

#[derive(Serialize)]
struct PlayerListResponse {
    status: &'static str,
    available: bool,
    reason: Option<&'static str>,
    players: Vec<serde_json::Value>,
    summary: PlayerSummary,
    services: PlayerServiceState,
}

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
    State(orchestrator): State<Arc<Orchestrator>>,
) -> Json<PlayerListResponse> {
    let state = orchestrator.state();
    let state = state.read().await;

    let db_connected = match &state.db {
        Some(pool) => pool.health_check().await,
        None => false,
    };

    Json(PlayerListResponse {
        status: "ok",
        available: false,
        reason: Some("Live player roster is not implemented yet."),
        players: Vec::new(),
        summary: PlayerSummary {
            online_count: 0,
            ready: state.base.is_running && state.cell.is_running,
        },
        services: PlayerServiceState {
            auth: state.auth.is_running,
            base: state.base.is_running,
            cell: state.cell.is_running,
            database: db_connected,
        },
    })
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

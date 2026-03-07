//! Game event WebSocket stream.
//!
//! Streams real-time game events to connected admin clients: player logins,
//! entity spawns, combat events, mission completions, etc.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::Response;

use cimmeria_services::orchestrator::Orchestrator;

/// WebSocket handler for game event streams.
///
/// Upgrades the HTTP connection to a WebSocket and begins streaming
/// game events. Clients can send filter messages to subscribe to specific
/// event types (e.g., only combat events, only login/logout).
pub async fn event_ws_handler(
    ws: WebSocketUpgrade,
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Response {
    ws.on_upgrade(handle_event_socket)
}

async fn handle_event_socket(mut _socket: WebSocket) {
    // TODO: Subscribe to game event bus
    // TODO: Send events as JSON messages
    // TODO: Handle client filter messages (event type filter)
    tracing::debug!("Event WebSocket connection established");
}

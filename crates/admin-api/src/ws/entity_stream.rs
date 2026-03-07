//! Entity property update WebSocket stream.
//!
//! Streams real-time entity property changes to connected admin clients.
//! Useful for live entity inspection in the ServerEd admin panel.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::Response;

use cimmeria_services::orchestrator::Orchestrator;

/// WebSocket handler for entity property streams.
///
/// Upgrades the HTTP connection to a WebSocket and begins streaming
/// entity property changes. Clients can send filter messages to subscribe
/// to specific entity IDs or property names.
pub async fn entity_ws_handler(
    ws: WebSocketUpgrade,
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Response {
    ws.on_upgrade(handle_entity_socket)
}

async fn handle_entity_socket(mut _socket: WebSocket) {
    // TODO: Subscribe to entity property change events
    // TODO: Send property updates as JSON messages
    // TODO: Handle client filter messages (subscribe to specific entities)
    tracing::debug!("Entity WebSocket connection established");
}

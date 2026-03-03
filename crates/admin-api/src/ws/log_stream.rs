//! Server log WebSocket stream.
//!
//! Streams real-time server log output to connected admin clients.
//! Replaces the original Python console on port 8989 with a more
//! structured approach.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::Response;

use cimmeria_services::orchestrator::Orchestrator;

/// WebSocket handler for log streams.
///
/// Upgrades the HTTP connection to a WebSocket and begins streaming
/// server log entries. Clients can send filter messages to set minimum
/// log level or filter by service/module.
pub async fn log_ws_handler(
    ws: WebSocketUpgrade,
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Response {
    ws.on_upgrade(handle_log_socket)
}

async fn handle_log_socket(mut _socket: WebSocket) {
    // TODO: Subscribe to tracing layer for log events
    // TODO: Send log entries as JSON messages
    // TODO: Handle client filter messages (log level, module filter)
    tracing::debug!("Log WebSocket connection established");
}

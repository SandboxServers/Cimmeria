//! WebSocket stream endpoints.
//!
//! Provides real-time data streams for the admin panel:
//! - Entity property updates
//! - Server log output
//! - Game event notifications

pub mod broadcast_layer;
pub mod entity_stream;
pub mod log_stream;
pub mod event_stream;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use cimmeria_services::orchestrator::Orchestrator;

/// Build WebSocket routes.
///
/// - `GET /entities` - Real-time entity property update stream
/// - `GET /logs` - Live server log output
/// - `GET /events` - Game event notifications
pub fn ws_routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/entities", get(entity_stream::entity_ws_handler))
        .route("/logs", get(log_stream::log_ws_handler))
        .route("/events", get(event_stream::event_ws_handler))
}

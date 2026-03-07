//! # cimmeria-admin-api
//!
//! REST and WebSocket admin API for the Cimmeria server emulator. Provides
//! endpoints for entity inspection, space management, content browsing,
//! player administration, configuration, and real-time event streaming.
//!
//! Designed to be consumed by the Tauri-based ServerEd desktop application
//! (replacing the original Qt-based ServerEd tool) and by any HTTP client
//! for server administration.

pub mod routes;
pub mod ws;
pub mod middleware;

use std::sync::Arc;

use axum::Router;

use cimmeria_services::orchestrator::Orchestrator;

/// Build the admin API router with all routes and middleware.
///
/// The router is configured with:
/// - `/api/` - REST endpoints for entities, spaces, content, players, config, auth
/// - `/ws/` - WebSocket streams for live entity, log, and event data
/// - CORS middleware for cross-origin access from the desktop app
///
/// # Arguments
///
/// * `orchestrator` - Shared reference to the service orchestrator, injected
///   as Axum state for all route handlers.
pub fn build_router(orchestrator: Arc<Orchestrator>) -> Router {
    let app = Router::new()
        .nest("/api", routes::api_routes())
        .nest("/ws", ws::ws_routes())
        .layer(middleware::cors_layer())
        .with_state(orchestrator);
    app
}

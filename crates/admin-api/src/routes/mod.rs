//! REST API route definitions.
//!
//! Each sub-module defines routes for a specific domain. All routes receive
//! the shared `Orchestrator` as Axum state for accessing server internals.

pub mod audit;
pub mod auth;
pub mod config;
pub mod content;
pub mod dev_session;
pub mod editor;
pub mod entities;
pub mod players;
pub mod spaces;
pub mod telemetry;

use std::sync::Arc;

use axum::Router;

use cimmeria_services::orchestrator::Orchestrator;

/// Build the combined REST API router.
///
/// All routes are nested under `/api/` by the parent router in `lib.rs`.
pub fn api_routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .nest("/entities", entities::routes())
        .nest("/spaces", spaces::routes())
        .nest("/content", content::routes())
        .nest("/players", players::routes())
        .nest("/config", config::routes())
        .nest("/editor", editor::routes())
        .nest("/auth", auth::routes())
        .nest("/audit", audit::routes())
        // The launcher-summary ingest is merged by name rather than
        // folded into `telemetry::routes()`, so each listener that serves
        // it says so.
        .nest(
            "/telemetry",
            telemetry::routes().merge(telemetry::launcher_summary_routes()),
        )
}

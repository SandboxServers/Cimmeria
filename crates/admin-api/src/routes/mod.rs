//! REST API route definitions.
//!
//! Each sub-module defines routes for a specific domain. All routes receive
//! the shared `Orchestrator` as Axum state for accessing server internals.

pub mod audit;
pub mod entities;
pub mod spaces;
pub mod content;
pub mod editor;
pub mod players;
pub mod config;
pub mod auth;

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
        // editor must come before content so /content/editor/... matches first
        .nest("/content/editor", editor::routes())
        .nest("/content", content::routes())
        .nest("/players", players::routes())
        .nest("/config", config::routes())
        .nest("/auth", auth::routes())
        .nest("/audit", audit::routes())
}

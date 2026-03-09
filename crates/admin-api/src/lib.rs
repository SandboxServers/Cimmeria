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

use axum::{Extension, Router};
use tokio::sync::broadcast;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use cimmeria_services::audit::{LoginEvent, LoginEventBuffer};
use cimmeria_services::orchestrator::Orchestrator;
use ws::broadcast_layer::{LogBuffer, LogEntry};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Config
        routes::config::get_config,
        routes::config::update_config,
        routes::config::get_status,
        routes::config::start_services,
        routes::config::stop_services,
        // Players
        routes::players::list_players,
        routes::players::get_player,
        routes::players::kick_player,
        // Spaces
        routes::spaces::list_spaces,
        routes::spaces::get_space,
        routes::spaces::create_space,
        // Content
        routes::content::list_categories,
        routes::content::list_items,
        routes::content::get_summary,
        routes::content::get_editor_pickers,
        routes::content::get_item,
        // Editor
        routes::editor::load_content,
        routes::editor::load_content_with_mission,
        routes::editor::save_content,
        routes::editor::delete_content,
        routes::editor::delete_content_with_mission,
        routes::editor::load_draft,
        routes::editor::load_draft_with_mission,
        routes::editor::save_draft,
        // Auth
        routes::auth::login,
        routes::auth::logout,
        routes::auth::current_user,
        // Entities
        routes::entities::list_entities,
        routes::entities::get_entity,
        routes::entities::set_entity_property,
    ),
    components(schemas(
        // Config
        routes::config::ConfigValues,
        routes::config::ConfigResponse,
        routes::config::ServiceStatus,
        routes::config::StatusResponse,
        // Players
        routes::players::PlayerSummary,
        routes::players::PlayerServiceState,
        routes::players::PlayerListResponse,
        // Spaces
        routes::spaces::SpaceRecord,
        routes::spaces::SpaceSummary,
        routes::spaces::SpaceListResponse,
        // Content
        routes::content::ContentSummary,
        routes::content::SpaceMissionCount,
        routes::content::ContentSummaryResponse,
        routes::content::EditorPickerOption,
        routes::content::EditorMissionOption,
        routes::content::EditorRegionOption,
        routes::content::EditorStepOption,
        routes::content::EditorPickersResponse,
        // Editor
        routes::editor::SaveRequest,
    )),
    tags(
        (name = "Config", description = "Server configuration and status"),
        (name = "Players", description = "Player roster and administration"),
        (name = "Spaces", description = "World space management"),
        (name = "Content", description = "Content engine browsing"),
        (name = "Editor", description = "Chain editor persistence"),
        (name = "Auth", description = "Admin API authentication"),
        (name = "Entities", description = "Entity inspection and manipulation"),
    ),
    info(
        title = "Cimmeria Admin API",
        version = "0.1.0",
        description = "REST API for the Cimmeria server emulator admin panel"
    )
)]
struct ApiDoc;

/// Build the admin API router with all routes and middleware.
///
/// The router is configured with:
/// - `/swagger-ui/` - Interactive API documentation (Swagger UI)
/// - `/api/` - REST endpoints for entities, spaces, content, players, config, auth
/// - `/ws/` - WebSocket streams for live entity, log, and event data
/// - CORS middleware for cross-origin access from the desktop app
///
/// # Arguments
///
/// * `orchestrator` - Shared reference to the service orchestrator, injected
///   as Axum state for all route handlers.
/// * `log_tx` - Broadcast sender for log entries, injected as an Axum Extension
///   for the `/ws/logs` WebSocket handler.
pub fn build_router(
    orchestrator: Arc<Orchestrator>,
    log_tx: broadcast::Sender<LogEntry>,
    log_buffer: LogBuffer,
    login_tx: broadcast::Sender<LoginEvent>,
    login_buffer: LoginEventBuffer,
) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api", routes::api_routes())
        .nest("/ws", ws::ws_routes())
        .layer(Extension(log_tx))
        .layer(Extension(log_buffer))
        .layer(Extension(login_tx))
        .layer(Extension(login_buffer))
        .layer(middleware::cors_layer())
        .with_state(orchestrator)
}

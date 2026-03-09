//! Server configuration endpoints.
//!
//! Provides REST endpoints for viewing and modifying server configuration
//! at runtime. Some settings require a service restart to take effect.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

use cimmeria_services::orchestrator::Orchestrator;

/// Server configuration values.
#[derive(Serialize, ToSchema)]
pub struct ConfigValues {
    pub auth_host: String,
    pub auth_port: u16,
    pub base_host: String,
    pub base_port: u16,
    pub cell_host: String,
    pub cell_port: u16,
    pub admin_port: u16,
    pub developer_mode: bool,
}

/// Response for GET /api/config.
#[derive(Serialize, ToSchema)]
pub struct ConfigResponse {
    pub status: String,
    pub config: ConfigValues,
}

/// Service health status.
#[derive(Serialize, ToSchema)]
pub struct ServiceStatus {
    pub auth: bool,
    pub base: bool,
    pub cell: bool,
    pub database: bool,
}

/// Response for GET /api/config/status.
#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub services: ServiceStatus,
}

/// Build configuration routes.
///
/// - `GET /` - Get current server configuration
/// - `POST /` - Update server configuration
/// - `GET /status` - Get server status and uptime
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/", get(get_config).post(update_config))
        .route("/status", get(get_status))
        .route("/start", post(start_services))
        .route("/stop", post(stop_services))
}

/// Get the current server configuration.
#[utoipa::path(
    get,
    path = "/api/config",
    responses(
        (status = 200, description = "Current server configuration", body = ConfigResponse)
    ),
    tag = "Config"
)]
pub async fn get_config(
    State(orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    let state = orchestrator.state();
    let state = state.read().await;
    Json(serde_json::json!({
        "status": "ok",
        "config": {
            "auth_host": state.config.auth_host,
            "auth_port": state.config.auth_port,
            "base_host": state.config.base_host,
            "base_port": state.config.base_port,
            "cell_host": state.config.cell_host,
            "cell_port": state.config.cell_port,
            "admin_port": state.config.admin_port,
            "developer_mode": state.config.developer_mode
        }
    }))
}

/// Update server configuration.
#[utoipa::path(
    post,
    path = "/api/config",
    responses(
        (status = 200, description = "Configuration update result", body = serde_json::Value)
    ),
    tag = "Config"
)]
pub async fn update_config(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Parse config changes from body, apply where possible
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented"
    }))
}

/// Get server status including uptime and service health.
#[utoipa::path(
    get,
    path = "/api/config/status",
    responses(
        (status = 200, description = "Server status and uptime", body = StatusResponse)
    ),
    tag = "Config"
)]
pub async fn get_status(
    State(orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    let uptime = orchestrator.uptime().await;
    let state = orchestrator.state();
    let state = state.read().await;

    let db_connected = match &state.db {
        Some(pool) => pool.health_check().await,
        None => false,
    };

    Json(serde_json::json!({
        "status": "ok",
        "uptime_seconds": uptime.as_secs(),
        "services": {
            "auth": state.auth.is_running,
            "base": state.base.is_running,
            "cell": state.cell.is_running,
            "database": db_connected
        }
    }))
}

/// Start all server services. Idempotent — no-op if already running.
#[utoipa::path(
    post,
    path = "/api/config/start",
    responses(
        (status = 200, description = "Start result", body = serde_json::Value)
    ),
    tag = "Config"
)]
pub async fn start_services(
    State(orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // Check if already running (idempotent)
    {
        let state = orchestrator.state();
        let state = state.read().await;
        if state.auth.is_running && state.base.is_running && state.cell.is_running {
            return Json(serde_json::json!({
                "status": "ok",
                "action": "none",
                "message": "All services already running."
            }));
        }
    }

    match orchestrator.start_all().await {
        Ok(()) => {
            tracing::info!("Services started via admin API");
            Json(serde_json::json!({
                "status": "ok",
                "action": "started",
                "message": "All services started successfully."
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to start services via admin API");
            Json(serde_json::json!({
                "status": "error",
                "action": "failed",
                "message": e.to_string()
            }))
        }
    }
}

/// Stop all server services. Idempotent — no-op if already stopped.
#[utoipa::path(
    post,
    path = "/api/config/stop",
    responses(
        (status = 200, description = "Stop result", body = serde_json::Value)
    ),
    tag = "Config"
)]
pub async fn stop_services(
    State(orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // Check if already stopped (idempotent)
    {
        let state = orchestrator.state();
        let state = state.read().await;
        if !state.auth.is_running && !state.base.is_running && !state.cell.is_running {
            return Json(serde_json::json!({
                "status": "ok",
                "action": "none",
                "message": "All services already stopped."
            }));
        }
    }

    orchestrator.stop_all().await;
    tracing::info!("Services stopped via admin API");

    Json(serde_json::json!({
        "status": "ok",
        "action": "stopped",
        "message": "All services stopped."
    }))
}

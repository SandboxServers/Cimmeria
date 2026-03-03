//! Server configuration endpoints.
//!
//! Provides REST endpoints for viewing and modifying server configuration
//! at runtime. Some settings require a service restart to take effect.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use cimmeria_services::orchestrator::Orchestrator;

/// Build configuration routes.
///
/// - `GET /` - Get current server configuration
/// - `POST /` - Update server configuration
/// - `GET /status` - Get server status and uptime
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/", get(get_config).post(update_config))
        .route("/status", get(get_status))
}

/// Get the current server configuration.
async fn get_config(
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
async fn update_config(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Parse config changes from body, apply where possible
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented"
    }))
}

/// Get server status including uptime and service health.
async fn get_status(
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

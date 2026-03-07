//! Admin API authentication endpoints.
//!
//! Provides JWT-based authentication for the admin API itself. This is
//! separate from game client authentication -- it protects the admin
//! panel from unauthorized access.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};

use cimmeria_services::orchestrator::Orchestrator;

/// Build auth routes.
///
/// - `POST /login` - Authenticate and receive a JWT token
/// - `POST /logout` - Invalidate the current token
/// - `GET /me` - Get the current authenticated admin user
pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(current_user))
}

/// Authenticate with the admin API and receive a JWT token.
async fn login(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Validate admin credentials, issue JWT
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "token": null
    }))
}

/// Invalidate the current JWT token.
async fn logout(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Add token to denylist
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented"
    }))
}

/// Get the currently authenticated admin user.
async fn current_user(
    State(_orchestrator): State<Arc<Orchestrator>>,
) -> Json<serde_json::Value> {
    // TODO: Extract user from JWT in Authorization header
    Json(serde_json::json!({
        "status": "ok",
        "message": "not implemented",
        "user": null
    }))
}

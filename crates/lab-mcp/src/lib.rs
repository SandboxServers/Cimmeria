//! In-server MCP endpoint for the Cimmeria live research lab (issue #687).
//!
//! An MCP server (streamable HTTP, via `rmcp`) with a fixed tool set that lets
//! an agent drive and inspect a running SGW server from Claude Code. It runs on
//! **its own** `TcpListener` — deliberately NOT mounted on the admin-api router,
//! which binds all interfaces with no auth and is published to players (#439).
//!
//! # Fail-closed
//!
//! The endpoint starts only when BOTH `CIMMERIA_LAB_MCP_BIND` and
//! `CIMMERIA_LAB_MCP_TOKEN` are set and the token is at least 32 bytes. There
//! is no default bind. See [`config`].
//!
//! # Auth + audit
//!
//! Every request must carry `Authorization: Bearer <token>`, compared in
//! constant time against the configured token ([`auth`]). Every tool call emits
//! exactly one `info` event on target `lab.tool_call` ([`audit`]).

pub mod audit;
pub mod auth;
pub mod config;
pub mod state;
mod tools;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use cimmeria_admin_api::ws::broadcast_layer::LogBuffer;
use cimmeria_services::orchestrator::Orchestrator;

pub use config::{LabMcpConfig, LabMcpStartup};
pub use state::LabState;

/// Start the lab MCP endpoint iff it is fully configured.
///
/// Returns the serve task's [`JoinHandle`] when started, or `None` when the
/// endpoint is disabled (env not set), refused (token too short), or the bind
/// failed — every non-start path is logged. This is the fail-closed entry
/// point `main.rs` calls next to the admin-API spawn.
pub async fn spawn_if_configured(
    orchestrator: Arc<Orchestrator>,
    log_buffer: LogBuffer,
) -> Option<JoinHandle<()>> {
    let cfg = match config::from_env() {
        LabMcpStartup::Disabled => {
            tracing::debug!(
                "lab MCP endpoint disabled ({} and {} must both be set)",
                config::ENV_BIND,
                config::ENV_TOKEN
            );
            return None;
        }
        LabMcpStartup::Refused(reason) => {
            tracing::error!(reason, "lab MCP endpoint refused to start");
            return None;
        }
        LabMcpStartup::Enabled(cfg) => cfg,
    };

    let listener = match TcpListener::bind(&cfg.bind).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(addr = %cfg.bind, error = %e, "lab MCP failed to bind — not starting");
            return None;
        }
    };
    tracing::info!(addr = %cfg.bind, "lab MCP endpoint listening");

    let state = LabState::new(orchestrator, log_buffer);
    let router = build_router(state, &cfg.token);

    let handle = tokio::spawn(async move {
        // `into_make_service_with_connect_info` is required so the bearer
        // middleware can read the peer `SocketAddr` for the audit trail.
        let make = router.into_make_service_with_connect_info::<SocketAddr>();
        if let Err(e) = axum::serve(listener, make).await {
            tracing::error!(error = %e, "lab MCP server error");
        }
    });
    Some(handle)
}

/// Build the axum router: the rmcp streamable-HTTP tool service at `/mcp`,
/// fronted by the bearer-token middleware. Split out so the wiring is testable
/// in isolation from `spawn_if_configured`.
fn build_router(state: LabState, token: &str) -> axum::Router {
    use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
    use rmcp::transport::streamable_http_server::StreamableHttpService;

    // One `LabTools` handler is built per MCP session; each gets its own clone
    // of the shared state handle.
    let service = StreamableHttpService::new(
        move || Ok(tools::LabTools::new(state.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let token: Arc<str> = Arc::from(token);
    axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn_with_state(
            token,
            auth::require_bearer,
        ))
}

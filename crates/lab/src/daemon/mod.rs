//! `cimmeria-lab --http <addr>`: one shared lab supervisor for every Claude
//! session on this machine, over MCP streamable HTTP.
//!
//! Stdio mode gives each session its own supervisor process, each with its
//! own watchdog, all driving the same `SGW.exe`; one relaunched a client
//! another had just closed. The daemon is one process with one supervisor,
//! so there is one watchdog and one owner of the client. ADR:
//! `docs/architecture/live-research-lab.md`; runbook:
//! `docs/guides/live-research-lab.md`.
//!
//! The HTTP edge is fail-closed, like the in-server lab endpoint
//! (`crates/lab-mcp`):
//! - loopback bind only ([`config::evaluate`]);
//! - `Authorization: Bearer <CIMMERIA_LAB_DAEMON_TOKEN>` on every request,
//!   32 bytes minimum ([`auth`]);
//! - rmcp's DNS-rebinding guard with loopback `Host` values only, and any
//!   browser `Origin` refused;
//! - one daemon per logon session ([`single_instance`]).
//!
//! - [`config`] — argument parsing and the fail-closed rules.
//! - [`auth`] — the bearer middleware.
//! - [`single_instance`] — the named-mutex guard and pidfile.
//! - [`log_file`] — `labd.log` with size rotation.

pub mod auth;
pub mod config;
pub mod log_file;
pub mod single_instance;

#[cfg(test)]
pub(crate) mod http_tests;

use std::sync::Arc;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use crate::server::LabServer;

/// Exit code when another daemon already runs (mutex held or port taken).
/// `tools/lab/daemon.ps1 run` stops its restart loop on it.
pub const EXIT_ALREADY_RUNNING: i32 = 3;
/// Exit code for a configuration refusal (bad bind, missing/short token).
pub const EXIT_REFUSED: i32 = 2;

/// The axum router: the MCP service at `/mcp` behind the bearer gate.
///
/// Every MCP session gets a clone of `server`, and every clone shares one
/// [`crate::supervisor::Supervisor`]: that sharing is the point of the daemon.
pub fn build_router(server: LabServer, token: &str) -> axum::Router {
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_allowed_hosts(config::LOOPBACK_HOSTS)
            // No browser has any business here: refuse every Origin.
            .enforce_origin_validation(),
    );
    let token: Arc<str> = Arc::from(token);
    axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn_with_state(
            token,
            auth::require_bearer,
        ))
}

/// Serve until Ctrl-C (or the task is stopped).
pub async fn serve(listener: tokio::net::TcpListener, router: axum::Router) -> anyhow::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("lab daemon stopping (ctrl-c)");
        })
        .await?;
    Ok(())
}

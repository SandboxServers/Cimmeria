//! `cimmeria-lab --http <addr>`: one shared lab supervisor for every Claude
//! session on this machine, over MCP streamable HTTP.
//!
//! Stdio mode gives each session its own supervisor process, each with its
//! own watchdog, all driving the same `SGW.exe`; one relaunched a client
//! another had just closed. The daemon is one process with one supervisor
//! per lab instance (`CIMMERIA_LAB_INSTANCES`, #1312), so each client has
//! one watchdog and one owner. ADR:
//! `docs/architecture/live-research-lab.md`; runbook:
//! `docs/guides/live-research-lab.md`.
//!
//! The HTTP edge is fail-closed, like the in-server lab endpoint
//! (`crates/lab-mcp`):
//! - loopback bind only ([`config::evaluate`]);
//! - `Authorization: Bearer <CIMMERIA_LAB_DAEMON_TOKEN>` on `/mcp`, `/status`
//!   and the one-use workbench opener, 32 bytes minimum ([`auth`]);
//! - rmcp's DNS-rebinding guard with loopback `Host` values only, and any
//!   browser `Origin` refused on `/mcp`;
//! - an independent cookie, local-origin and CSRF boundary on `/workbench/`;
//! - one daemon per logon session ([`single_instance`]).
//!
//! - [`config`] — argument parsing and the fail-closed rules.
//! - [`auth`] — the bearer middleware.
//! - [`single_instance`] — the named-mutex guard and pidfile.
//! - [`log_file`] — `labd.log` with size rotation.
//! - [`status`] — `GET /status`, a read-only daemon and instance summary.

pub mod auth;
pub mod config;
pub mod log_file;
pub mod single_instance;
pub mod status;
pub mod workbench;

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

/// The axum router: MCP and status behind the bearer gate, with an isolated
/// workbench session boundary.
///
/// Every MCP session gets a clone of `server`, and every clone shares one
/// [`crate::supervisor::Supervisor`]: that sharing is the point of the daemon.
pub fn build_router(server: LabServer, token: &str) -> axum::Router {
    build_router_with_sessions(
        server,
        token,
        Arc::new(workbench::session::SessionStore::default()),
    )
}

fn build_router_with_sessions(
    server: LabServer,
    token: &str,
    sessions: Arc<workbench::session::SessionStore>,
) -> axum::Router {
    if let Some(atlas) = server.livewire_atlas() {
        tracing::info!(
            libraries = atlas.libraries.len(),
            goal_art = atlas.library("gGreenBlack1").is_some(),
            "Livewire hit atlas loaded"
        );
    }
    let server_for_status = server.clone();
    let instances = server.instances().clone();
    let started = Arc::new(status::Started {
        at_ms: crate::supervisor::now_ms(),
        pid: std::process::id(),
    });
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_allowed_hosts(config::LOOPBACK_HOSTS)
            // No browser has any business here: refuse every Origin.
            .enforce_origin_validation(),
    );
    let token: Arc<str> = Arc::from(token);
    let bearer = axum::Router::new()
        .nest_service("/mcp", service)
        .route(
            "/status",
            axum::routing::get(status::handler).with_state((server_for_status, started)),
        )
        .layer(axum::middleware::from_fn_with_state(
            token.clone(),
            auth::require_bearer,
        ));
    bearer.merge(workbench::router(sessions, token, instances))
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

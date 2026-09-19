//! `cimmeria-lab` — Live Research Lab supervisor, phase 1.
//!
//! A minimal stdio MCP server that proxies three tools
//! (`client_lua_eval`, `client_module_info`, `client_mem_read`) to
//! the injected client bridge over a token-gated framed-JSON-RPC TCP
//! channel. Later phases add process lifecycle, screenshots,
//! autologin, and crash recovery (see
//! `docs/architecture/live-research-lab.md` §3.4).
//!
//! Config comes from the environment:
//!
//! - `CIMMERIA_LAB_BRIDGE` — bridge address for attaching to an
//!   already-running client, default `127.0.0.1:8770`. Once the
//!   supervisor launches a client itself (`lab_client_start`) it
//!   re-points the bridge at the fresh per-launch token.
//! - `CIMMERIA_LAB_TOKEN` — the 64-hex token for that pre-existing
//!   client (unused once the supervisor starts one).
//! - `CIMMERIA_LAB_INSTALL_DIR` — game install dir (for the supervisor
//!   lifecycle tools). `CIMMERIA_LAB_DLL` overrides the DLL path.
//! - `CIMMERIA_LAB_BRIDGE_BIND` / `_PORT`, `CIMMERIA_LAB_UPLOAD_ENDPOINT`
//!   — written into the session file the supervisor generates.

use std::sync::Arc;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

mod client;
mod server;
mod supervisor;
mod timeline;

use client::BridgeClient;
use server::LabServer;
use supervisor::{Supervisor, SupervisorConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Logs go to stderr — stdout is the MCP transport.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let addr =
        std::env::var("CIMMERIA_LAB_BRIDGE").unwrap_or_else(|_| "127.0.0.1:8770".to_string());
    let token = std::env::var("CIMMERIA_LAB_TOKEN").unwrap_or_default();
    if token.is_empty() {
        tracing::warn!(
            "CIMMERIA_LAB_TOKEN is unset; attaching to a pre-existing client will \
             fail until lab_client_start mints its own token"
        );
    }
    let config = SupervisorConfig::from_env();
    tracing::info!(%addr, install_dir = ?config.install_dir, "cimmeria-lab supervisor starting");

    let bridge = Arc::new(BridgeClient::new(addr, token));
    let supervisor = Arc::new(Supervisor::new(bridge, config));
    let service = LabServer::new(supervisor)
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("serve error: {e:?}"))?;

    service.waiting().await?;
    Ok(())
}

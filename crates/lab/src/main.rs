//! `cimmeria-lab` — Live Research Lab supervisor, phase 1.
//!
//! A minimal stdio MCP server that proxies three tools
//! (`client_lua_eval`, `client_module_info`, `client_mem_read`) to
//! the injected client bridge over a token-gated framed-JSON-RPC TCP
//! channel. Later phases add process lifecycle, screenshots,
//! autologin, and crash recovery (see
//! `docs/architecture/live-research-lab.md` §3.4).
//!
//! Config comes from the environment for phase 1; the supervisor will
//! own it end to end once it writes the session file itself:
//!
//! - `CIMMERIA_LAB_BRIDGE` — bridge address, default `127.0.0.1:8770`.
//! - `CIMMERIA_LAB_TOKEN` — the 64-hex token the bridge expects.

use std::sync::Arc;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

mod client;
mod server;

use client::BridgeClient;
use server::LabServer;

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
        tracing::warn!("CIMMERIA_LAB_TOKEN is unset; the bridge will reject every call");
    }
    tracing::info!(%addr, "cimmeria-lab starting; proxying to client bridge");

    let bridge = Arc::new(BridgeClient::new(addr, token));
    let service = LabServer::new(bridge)
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("serve error: {e:?}"))?;

    service.waiting().await?;
    Ok(())
}

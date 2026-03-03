//! Cimmeria server binary.
//!
//! Starts all three services (Auth HTTP on 13001, Base UDP on 32832,
//! Cell UDP on 32833) and waits for Ctrl-C.
//!
//! # Environment variables
//!
//! | Variable | Default | Description |
//! |---|---|---|
//! | `AUTH_HOST` | `0.0.0.0` | Auth service bind address |
//! | `AUTH_PORT` | `13001` | Auth service port (BaseApp connections) |
//! | `LOGON_PORT` | `8081` | Auth HTTP port (SOAP client login) |
//! | `BASE_HOST` | `0.0.0.0` | BaseApp UDP bind address |
//! | `BASE_EXTERNAL` | `127.0.0.1` | BaseApp address advertised to game clients |
//! | `BASE_PORT` | `32832` | BaseApp UDP port |
//! | `CELL_PORT` | `50000` | CellApp port |
//! | `DB_URL` | `host=localhost port=5433 user=w-testing password=w-testing dbname=sgw` | PostgreSQL connection string |
//! | `PROTOCOL_DIGEST` | `58AFA196...` | 32-char hex digest sent in auth response |
//! | `DEVELOPER_MODE` | `true` | Enable relaxed auth / multi-login |
//! | `RUST_LOG` | `info` | Log filter (e.g. `debug`, `cimmeria_services=trace`) |
//!
//! # Example
//!
//! ```sh
//! RUST_LOG=debug cargo run -p cimmeria-server
//! ```

use cimmeria_common::ServerConfig;
use cimmeria_services::orchestrator::Orchestrator;

#[tokio::main]
async fn main() {
    // Initialise tracing — respects RUST_LOG env var.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = config_from_env();

    tracing::info!(
        auth_port = config.auth_port,
        base_port = config.base_port,
        "Starting Cimmeria server"
    );

    let orch = Orchestrator::new(config);

    if let Err(e) = orch.start_all().await {
        tracing::error!("Failed to start services: {e}");
        std::process::exit(1);
    }

    tracing::info!("Server ready. Press Ctrl-C to stop.");

    // Wait for Ctrl-C.
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl-C");

    tracing::info!("Shutting down…");
    orch.stop_all().await;
    tracing::info!("Goodbye.");
}

/// Build a [`ServerConfig`] from environment variables, falling back to defaults.
fn config_from_env() -> ServerConfig {
    let mut cfg = ServerConfig::default();

    if let Ok(v) = std::env::var("AUTH_HOST") {
        cfg.auth_host = v;
    }
    if let Ok(v) = std::env::var("AUTH_PORT") {
        if let Ok(p) = v.parse() {
            cfg.auth_port = p;
        }
    }
    if let Ok(v) = std::env::var("LOGON_PORT") {
        if let Ok(p) = v.parse() {
            cfg.logon_port = p;
        }
    }
    if let Ok(v) = std::env::var("BASE_HOST") {
        cfg.base_host = v;
    }
    if let Ok(v) = std::env::var("BASE_EXTERNAL") {
        cfg.base_external_host = v;
    }
    if let Ok(v) = std::env::var("BASE_PORT") {
        if let Ok(p) = v.parse() {
            cfg.base_port = p;
        }
    }
    if let Ok(v) = std::env::var("CELL_PORT") {
        if let Ok(p) = v.parse() {
            cfg.cell_port = p;
        }
    }
    if let Ok(v) = std::env::var("DB_URL") {
        cfg.db_connection_string = v;
    }
    if let Ok(v) = std::env::var("PROTOCOL_DIGEST") {
        cfg.protocol_digest = v;
    }
    if let Ok(v) = std::env::var("DEVELOPER_MODE") {
        cfg.developer_mode = matches!(v.to_lowercase().as_str(), "1" | "true" | "yes");
    }

    cfg
}

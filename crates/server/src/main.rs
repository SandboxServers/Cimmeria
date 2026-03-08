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

use std::fs;
use std::path::Path;
use std::sync::Arc;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

use cimmeria_common::ServerConfig;
use cimmeria_services::orchestrator::Orchestrator;

#[tokio::main]
async fn main() {
    // Initialise layered tracing — guards must live until shutdown.
    let _guards = init_logging();

    tracing::trace!(pid = std::process::id(), "Process spawned");

    let config = config_from_env();

    tracing::trace!(
        auth_host = %config.auth_host,
        auth_port = config.auth_port,
        logon_port = config.logon_port,
        base_host = %config.base_host,
        base_external = %config.base_external_host,
        base_port = config.base_port,
        cell_port = config.cell_port,
        developer_mode = config.developer_mode,
        "Configuration loaded"
    );

    tracing::info!(
        auth_port = config.auth_port,
        base_port = config.base_port,
        "Starting Cimmeria server"
    );

    let admin_port = config.admin_port;

    tracing::trace!("Creating orchestrator");
    let orch = Arc::new(Orchestrator::new(config));

    tracing::trace!("Calling start_all");
    if let Err(e) = orch.start_all().await {
        tracing::error!("Failed to start services: {e}");
        tracing::trace!(pid = std::process::id(), "Process exiting with code 1");
        std::process::exit(1);
    }

    // Start the admin API (REST + WebSocket) on the configured port.
    let admin_router = cimmeria_admin_api::build_router(Arc::clone(&orch));
    let admin_addr = format!("0.0.0.0:{admin_port}");
    let admin_listener = match tokio::net::TcpListener::bind(&admin_addr).await {
        Ok(listener) => {
            tracing::info!(addr = %admin_addr, "Admin API listening");
            listener
        }
        Err(e) => {
            tracing::error!(addr = %admin_addr, "Failed to bind admin API: {e}");
            tracing::trace!(pid = std::process::id(), "Process exiting with code 1");
            std::process::exit(1);
        }
    };
    tokio::spawn(async move {
        if let Err(e) = axum::serve(admin_listener, admin_router).await {
            tracing::error!("Admin API server error: {e}");
        }
    });

    tracing::info!("Server ready. Press Ctrl-C to stop.");

    // Wait for Ctrl-C.
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl-C");

    tracing::info!("Shutting down…");
    tracing::trace!("Calling stop_all");
    orch.stop_all().await;
    tracing::trace!("stop_all complete");
    tracing::info!("Goodbye.");
    tracing::trace!(pid = std::process::id(), "Process exiting with code 0");
}

// ── Logging ──────────────────────────────────────────────────────────────────

/// Archive any `.log` files from a previous session into `logs/archive/<timestamp>/`.
fn archive_previous_logs() {
    let logs_dir = Path::new("logs");
    if !logs_dir.exists() {
        return;
    }

    let entries: Vec<_> = fs::read_dir(logs_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "log")
        })
        .collect();

    if entries.is_empty() {
        return;
    }

    let ts = chrono_timestamp();
    let archive_dir = logs_dir.join("archive").join(&ts);
    if let Err(e) = fs::create_dir_all(&archive_dir) {
        eprintln!("Failed to create archive directory {}: {e}", archive_dir.display());
        return;
    }

    for entry in &entries {
        let src = entry.path();
        let dst = archive_dir.join(entry.file_name());
        if let Err(e) = fs::rename(&src, &dst) {
            eprintln!("Failed to archive {}: {e}", src.display());
        }
    }

    eprintln!("Archived {} log file(s) to {}", entries.len(), archive_dir.display());
}

/// Generate a filesystem-safe timestamp like `2026-03-03T14-30-22`.
fn chrono_timestamp() -> String {
    use std::time::SystemTime;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Simple UTC breakdown (no chrono dependency needed).
    let secs_per_day: u64 = 86400;
    let days = now / secs_per_day;
    let day_secs = now % secs_per_day;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;

    // Days since epoch to Y-M-D (simplified Gregorian).
    let (year, month, day) = days_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's `civil_from_days`.
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m, d)
}

/// Initialise a layered tracing subscriber.
///
/// Returns `WorkerGuard`s that **must** be held alive in `main()` — dropping
/// them flushes and closes the log files.
///
/// Layers:
/// - **Console** (stdout): coloured, human-readable, level from `RUST_LOG` (default `info`).
/// - **`logs/server.log`**: JSON, all modules at `debug`.
/// - **`logs/auth.log`**: plain text, `cimmeria_services::auth` at `trace`.
/// - **`logs/base.log`**: plain text, base/mercury modules at `trace`.
fn init_logging() -> Vec<WorkerGuard> {
    // Move previous session's logs into archive/.
    archive_previous_logs();

    // Ensure logs/ directory exists.
    let _ = fs::create_dir_all("logs");

    let mut guards = Vec::new();

    // ── server.log (JSON, all modules, debug) ────────────────────────────
    let server_file = tracing_appender::rolling::never("logs", "server.log");
    let (server_writer, guard) = tracing_appender::non_blocking(server_file);
    guards.push(guard);

    let server_layer = fmt::layer()
        .json()
        .with_writer(server_writer)
        .with_target(true)
        .with_filter(EnvFilter::new("trace"));

    // ── auth.log (plain text, auth module, trace) ────────────────────────
    let auth_file = tracing_appender::rolling::never("logs", "auth.log");
    let (auth_writer, guard) = tracing_appender::non_blocking(auth_file);
    guards.push(guard);

    let auth_layer = fmt::layer()
        .with_writer(auth_writer)
        .with_ansi(false)
        .with_target(true)
        .with_filter(EnvFilter::new("off,cimmeria_services::auth=trace"));

    // ── base.log (plain text, base/mercury modules, trace) ───────────────
    let base_file = tracing_appender::rolling::never("logs", "base.log");
    let (base_writer, guard) = tracing_appender::non_blocking(base_file);
    guards.push(guard);

    let base_layer = fmt::layer()
        .with_writer(base_writer)
        .with_ansi(false)
        .with_target(true)
        .with_filter(EnvFilter::new(
            "off,cimmeria_services::base=trace,cimmeria_services::mercury=trace,cimmeria_mercury=trace",
        ));

    // ── Console (stdout, coloured, RUST_LOG or info) ─────────────────────
    let console_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let console_layer = fmt::layer()
        .with_filter(console_filter);

    // Assemble the subscriber.
    tracing_subscriber::registry()
        .with(console_layer)
        .with(server_layer)
        .with(auth_layer)
        .with(base_layer)
        .init();

    guards
}

// ── Config ───────────────────────────────────────────────────────────────────

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
    if let Ok(v) = std::env::var("ADMIN_PORT") {
        if let Ok(p) = v.parse() {
            cfg.admin_port = p;
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

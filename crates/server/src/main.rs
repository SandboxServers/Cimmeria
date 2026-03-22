//! Cimmeria server binary.
//!
//! Starts all three game services (Auth HTTP on 13001, Base UDP on 32832,
//! Cell UDP on 32833), the admin REST API (HTTP on 8443), and waits for
//! Ctrl-C.
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
//! | `ADMIN_PORT` | `8443` | Admin REST API port |
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

use tokio::sync::broadcast;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

use cimmeria_admin_api::ws::broadcast_layer::{BroadcastLayer, LogBuffer, LogEntry};
use cimmeria_common::ServerConfig;
use cimmeria_services::audit::{LoginEvent, LoginEventBuffer};
use cimmeria_services::orchestrator::Orchestrator;

mod cosmos_log;

#[tokio::main]
async fn main() {
    // Create log broadcast channel and ring buffer (for WebSocket log streaming).
    let (log_tx, _) = broadcast::channel::<LogEntry>(2048);
    let log_buffer = LogBuffer::new();

    // Create login audit channel and ring buffer.
    let (login_tx, _) = broadcast::channel::<LoginEvent>(256);
    let login_buffer = LoginEventBuffer::new();

    // Generate a session ID for this server run.
    let session_id = generate_session_id();

    // Cosmos DB log sink (optional — requires COSMOS_LOG_ENDPOINT + COSMOS_LOG_KEY).
    let cosmos_parts = match (
        std::env::var("COSMOS_LOG_ENDPOINT"),
        std::env::var("COSMOS_LOG_KEY"),
    ) {
        (Ok(endpoint), Ok(key)) => {
            let (layer, receiver) = cosmos_log::create_cosmos_log_sink(session_id.clone());
            Some((layer, receiver, endpoint, key))
        }
        _ => None,
    };

    // Split out the layer (needed for subscriber) from the receiver (spawned after).
    let (cosmos_layer, cosmos_writer_parts) = match cosmos_parts {
        Some((layer, receiver, endpoint, key)) => (Some(layer), Some((receiver, endpoint, key))),
        None => (None, None),
    };

    // Initialise layered tracing — guards must live until shutdown.
    let _guards = init_logging(log_tx.clone(), log_buffer.clone(), cosmos_layer);

    // Spawn the Cosmos DB writer task now that the subscriber is active.
    if let Some((receiver, endpoint, key)) = cosmos_writer_parts {
        tokio::spawn(cosmos_log::cosmos_log_writer(receiver, endpoint, key));
        eprintln!("[cosmos-log] Streaming to Cosmos DB (session: {session_id})");
    }

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
    let mut orch = Orchestrator::new(config);
    orch.set_login_event_channel(login_tx.clone(), login_buffer.clone());
    let orch = Arc::new(orch);

    tracing::trace!("Calling start_all");
    if let Err(e) = orch.start_all().await {
        tracing::error!("Failed to start services: {e}");
        tracing::trace!(pid = std::process::id(), "Process exiting with code 1");
        std::process::exit(1);
    }

    // Spawn background audit writer to persist login events to the database.
    {
        let state = orch.state();
        let state = state.read().await;
        let audit_pool = state.db.as_ref().map(|db| db.pool().clone());
        drop(state);
        if let Some(pool) = audit_pool {
            let audit_rx = login_tx.subscribe();
            tokio::spawn(audit_writer_loop(pool, audit_rx));
        }
    }

    // Start the admin API (REST + WebSocket) on the configured port.
    let admin_router = cimmeria_admin_api::build_router(
        Arc::clone(&orch),
        log_tx.clone(),
        log_buffer,
        login_tx.clone(),
        login_buffer.clone(),
    );
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

// ── Audit persistence ────────────────────────────────────────────────────────

/// Background task that persists [`LoginEvent`]s to the `login_audit` table.
///
/// Tolerates DB unavailability — logs a warning and keeps running.
async fn audit_writer_loop(
    pool: sqlx::PgPool,
    mut rx: broadcast::Receiver<LoginEvent>,
) {
    tracing::info!("Audit writer started");
    loop {
        match rx.recv().await {
            Ok(event) => {
                if let Err(e) = sqlx::query(
                    "INSERT INTO login_audit (event_time, account_name, account_id, ip_address, phase, outcome, shard, detail) \
                     VALUES (TO_TIMESTAMP($1::DOUBLE PRECISION / 1000), $2, $3, $4::INET, $5, $6, $7, $8)"
                )
                .bind(event.timestamp_ms as f64)
                .bind(&event.account_name)
                .bind(event.account_id.map(|id| id as i32))
                .bind(&event.ip_address)
                .bind(&event.phase)
                .bind(&event.outcome)
                .bind(&event.shard)
                .bind(&event.detail)
                .execute(&pool)
                .await
                {
                    tracing::warn!(error = %e, "Failed to write login audit event");
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(skipped = n, "Audit writer lagged, events dropped");
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    tracing::info!("Audit writer shutting down");
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
/// - **`logs/server.log`**: JSON, all modules at `info` (high-level milestones only).
/// - **`logs/auth.log`**: auth module at `trace`.
/// - **`logs/base.log`**: connection lifecycle (service, connect_loop, login, tick_sync, helpers) at `trace`.
/// - **`logs/world_entry.log`**: player world entry, DB queries, appearance at `trace`.
/// - **`logs/character.log`**: char list, creation, visuals, PAK serving at `trace`.
/// - **`logs/protocol.log`**: wire-level packet building, encryption at `trace`.
/// - **`logs/aoi.log`**: AoI tick, entity create/destroy, spatial grid at `trace`.
/// - **`logs/combat.log`**: damage, abilities, NPC AI, death/respawn at `trace`.
/// - **`logs/content.log`**: content chain triggers, conditions, actions at `trace`.
/// - **`logs/missions.log`**: mission accept/complete/abandon, objectives at `trace`.
/// - **`logs/interactions.log`**: dialogs, vendors, trainers, loot, chat, mail at `trace`.
/// - **`logs/spawner.log`**: NPC spawning, stargate travel at `trace`.
/// - **`logs/dispatch.log`**: method routing (cell + base method dispatch) at `trace`.
/// - **WebSocket broadcast**: `debug` for admin panel streaming.
/// - **Cosmos DB** (optional): `debug` minus per-packet noise.
///
/// Correlation: most tracing calls include `entity_id`, `player_id`, or
/// `witness_id` as structured fields.  To follow a player across files:
/// `grep entity_id=42 logs/*.log`.
fn init_logging(
    log_tx: broadcast::Sender<LogEntry>,
    log_buffer: LogBuffer,
    cosmos_layer: Option<cosmos_log::CosmosLogLayer>,
) -> Vec<WorkerGuard> {
    // Move previous session's logs into archive/.
    archive_previous_logs();

    // Ensure logs/ directory exists.
    let _ = fs::create_dir_all("logs");

    let mut guards = Vec::new();

    type BoxLayer = Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>;

    // Helper: create a plain-text, non-ANSI log file layer with a specific filter.
    // Returns a boxed layer to avoid deeply nested generic types that cause
    // the Rust type-checker to consume 50+ GB of RAM.
    macro_rules! log_layer {
        ($filename:expr, $filter:expr) => {{
            let file = tracing_appender::rolling::never("logs", $filename);
            let (writer, guard) = tracing_appender::non_blocking(file);
            guards.push(guard);
            Box::new(
                fmt::layer()
                    .with_writer(writer)
                    .with_ansi(false)
                    .with_target(true)
                    .with_filter(EnvFilter::new($filter)),
            ) as BoxLayer
        }};
    }

    // ── Collect all layers into a Vec<BoxLayer> ─────────────────────────
    let mut layers: Vec<BoxLayer> = Vec::new();

    // ── Console (stdout, coloured, RUST_LOG or info) ─────────────────────
    let console_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    layers.push(Box::new(fmt::layer().with_filter(console_filter)));

    // ── server.log (JSON, all modules, info) ─────────────────────────────
    let server_file = tracing_appender::rolling::never("logs", "server.log");
    let (server_writer, guard) = tracing_appender::non_blocking(server_file);
    guards.push(guard);
    layers.push(Box::new(
        fmt::layer()
            .json()
            .with_writer(server_writer)
            .with_target(true)
            .with_filter(EnvFilter::new("info")),
    ));

    // ── Per-system log files ─────────────────────────────────────────────
    layers.push(log_layer!("auth.log", "off,cimmeria_services::auth=trace"));

    layers.push(log_layer!(
        "base.log",
        "off,\
         cimmeria_services::base::service=trace,\
         cimmeria_services::base::connect_loop=trace,\
         cimmeria_services::base::login=trace,\
         cimmeria_services::base::tick_sync=trace,\
         cimmeria_services::base::helpers=trace"
    ));

    layers.push(log_layer!(
        "world_entry.log",
        "off,\
         cimmeria_services::base::world_entry=trace,\
         cimmeria_services::base::world_entry_player=trace,\
         cimmeria_services::base::world_entry_appearance=trace"
    ));

    layers.push(log_layer!(
        "character.log",
        "off,\
         cimmeria_services::base::character=trace,\
         cimmeria_services::base::character_create=trace,\
         cimmeria_services::base::chardef=trace,\
         cimmeria_services::base::cooked_data=trace,\
         cimmeria_services::base::resources=trace"
    ));

    layers.push(log_layer!(
        "protocol.log",
        "off,\
         cimmeria_services::mercury=trace,\
         cimmeria_mercury=trace"
    ));

    layers.push(log_layer!(
        "aoi.log",
        "off,\
         cimmeria_services::cell::service=trace,\
         cimmeria_services::cell::space_manager=trace"
    ));

    layers.push(log_layer!(
        "combat.log",
        "off,\
         cimmeria_services::cell::combat=trace,\
         cimmeria_services::cell::abilities=trace"
    ));

    layers.push(log_layer!(
        "content.log",
        "off,cimmeria_services::cell::content=trace"
    ));

    layers.push(log_layer!(
        "missions.log",
        "off,cimmeria_services::cell::missions=trace"
    ));

    layers.push(log_layer!(
        "interactions.log",
        "off,\
         cimmeria_services::cell::interactions=trace,\
         cimmeria_services::cell::chat=trace,\
         cimmeria_services::cell::mail=trace"
    ));

    layers.push(log_layer!(
        "spawner.log",
        "off,\
         cimmeria_services::cell::spawner=trace,\
         cimmeria_services::cell::gate_travel=trace"
    ));

    layers.push(log_layer!(
        "dispatch.log",
        "off,\
         cimmeria_services::cell::dispatch=trace,\
         cimmeria_services::base::dispatch=trace"
    ));

    // ── WebSocket broadcast (debug+, all modules) ─────────────────────
    layers.push(Box::new(
        BroadcastLayer::new(log_tx, log_buffer)
            .with_filter(EnvFilter::new(
                "debug,tungstenite=info,tokio_tungstenite=info,hyper=info",
            )),
    ));

    // ── Cosmos DB (optional, filtered to useful events only) ─────────
    if let Some(layer) = cosmos_layer {
        layers.push(Box::new(
            layer.with_filter(EnvFilter::new(
                "debug,\
                 cimmeria_services::base::connect_loop=info,\
                 cimmeria_mercury::encryption=info,\
                 cimmeria_services::base::tick_sync=info,\
                 tungstenite=off,tokio_tungstenite=off,hyper=off",
            )),
        ));
    }

    // Assemble the subscriber — one `.with()` call on the whole Vec.
    tracing_subscriber::registry()
        .with(layers)
        .init();

    guards
}

// ── Config ───────────────────────────────────────────────────────────────────

/// Generate a human-readable session ID: `2026-03-13T22-56-15_a3f1`.
///
/// Combines the archive timestamp format with a short random suffix to
/// avoid collisions on rapid restarts. Used as the Cosmos DB partition key.
fn generate_session_id() -> String {
    use rand::Rng;
    let ts = chrono_timestamp();
    let suffix: u16 = rand::rng().random();
    format!("{}_{:04x}", ts, suffix)
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

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
//! | `CIMMERIA_TELEMETRY_HMAC_SECRET` | unset | HMAC-SHA256 secret for the launcher dev-session token mint at `/api/auth/dev-session` and the launcher upload endpoints at `/api/telemetry/upload-{chunk,bundle}`. See [docs/operations/telemetry.md](../../../docs/operations/telemetry.md). Unset ⇒ endpoint returns 500. |
//! | `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` | `http://localhost:8443/api/telemetry` | Upload endpoint URL handed back to the launcher in the dev-session response. The default works when the launcher and server share a host; cross-host deployments MUST override (e.g. to a public LAN URL, or through the Cloudflare Tunnel). See [docs/operations/telemetry.md](../../../docs/operations/telemetry.md). |
//! | `CIMMERIA_TELEMETRY_KILL_SWITCH` | unset | Set to `1` to pause telemetry ingest (every mint returns 503 + Retry-After). |
//! | `OTEL_EXPORTER_OTLP_ENDPOINT` | unset | OTLP collector endpoint (e.g. `http://otel-collector:4317`). Unset ⇒ OTLP exporter disabled; logs and Mercury packet events never leave the process via OTLP. See [docs/operations/signoz-deployment.md](../../../docs/operations/signoz-deployment.md). |
//! | `OTEL_EXPORTER_OTLP_PROTOCOL` | `grpc` | `grpc` (default) or `http/protobuf`. |
//! | `OTEL_SERVICE_NAME` | `cimmeria-server` | Shown as `service.name` in SigNoz's service map. |
//! | `OTEL_RESOURCE_ATTRIBUTES` | unset | Comma-separated `k=v` resource attrs piped onto every span. Common: `deployment.environment=colo,service.namespace=cimmeria`. Note: `deployment.environment` is also defaulted from `CIMMERIA_DEPLOY_ENV` below; this env var overrides it via the SDK's resource merge. |
//! | `OTEL_TRACES_SAMPLER` | `always_on` | `always_on`, `always_off`, or `traceidratio` with `OTEL_TRACES_SAMPLER_ARG`. |
//! | `CIMMERIA_DEPLOY_ENV` | `dev` | Sets `deployment.environment` on every span/log/metric resource. Typical values: `dev`, `staging`, `colo`. SigNoz dashboards split aggregates on this so colo production data isn't polluted by dev-laptop noise. |
//!
//! # Example
//!
//! ```sh
//! RUST_LOG=debug cargo run -p cimmeria-server
//! ```

use std::sync::Arc;

use tokio::sync::broadcast;

use cimmeria_admin_api::ws::broadcast_layer::{LogBuffer, LogEntry};
use cimmeria_common::ServerConfig;
use cimmeria_services::audit::{LoginEvent, LoginEventBuffer};
use cimmeria_services::orchestrator::Orchestrator;

mod logging;
mod otel;

#[tokio::main]
async fn main() {
    let server_start = std::time::Instant::now();

    // Initialise Discord notifications BEFORE tracing — the tracing
    // layer needs the sender handle, and the panic hook needs the
    // global runtime in place.
    //
    // Failure policy: a *missing* config file is a soft-fail (Discord
    // disabled, server starts normally — the typical local-dev case).
    // An *invalid* config file (parse error, unknown event key, bad
    // webhook URL) is a hard-fail with exit code 2 — the operator
    // edited the file, the typo should not silently hide misconfig.
    let discord_config_path =
        std::env::var("DISCORD_CONFIG_PATH").unwrap_or_else(|_| "config/discord.toml".to_string());
    let discord_runtime = match cimmeria_discord::init(&discord_config_path) {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!(
                "[discord] config error at `{discord_config_path}`: {e}\n\
                 Refusing to start with an invalid Discord config. Fix the file \
                 or remove it to run with Discord disabled."
            );
            std::process::exit(2);
        }
    };
    // True only when the config loaded as an enabled, configured runtime
    // (i.e. file present + parsed). Used to gate the post-Ctrl-C drain.
    let discord_enabled = discord_runtime.config.load().enabled;
    cimmeria_discord::install_panic_hook();

    // Create log broadcast channel and ring buffer (for WebSocket log streaming).
    let (log_tx, _) = broadcast::channel::<LogEntry>(2048);
    let log_buffer = LogBuffer::new();

    // Create login audit channel and ring buffer.
    let (login_tx, _) = broadcast::channel::<LoginEvent>(256);
    let login_buffer = LoginEventBuffer::new();

    // OTLP exporter (optional — requires OTEL_EXPORTER_OTLP_ENDPOINT).
    // Three layers come back: a trace layer (spans + span events), a
    // log layer for the high-signal `cimmeria-server` index (auth,
    // content, combat, missions), and a separate log layer for the
    // high-noise `cimmeria-network` index (mercury_packet, bundle
    // decode, cell-arms dispatch, tick-sync heartbeats). The split is
    // resource-tagged at the OTLP provider level — see
    // `otel::is_network_noise_target` for the routing predicate. The
    // guard is bound at this scope so it drops *after* the
    // orchestrator's stop_all returns, flushing all three in-flight
    // batches on clean shutdown.
    let (otel_trace_layer, otel_log_layer, otel_network_log_layer, _otel_guard) = match otel::init()
    {
        Some((trace, log, network, guard)) => (Some(trace), Some(log), Some(network), Some(guard)),
        None => (None, None, None, None),
    };

    // Initialise layered tracing — guards must live until shutdown.
    let _guards = logging::init_logging(
        log_tx.clone(),
        log_buffer.clone(),
        otel_trace_layer,
        otel_log_layer,
        otel_network_log_layer,
    );

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
    // Capture ports for the Discord startup embed before `config` moves
    // into the orchestrator.
    let (auth_port_for_discord, base_port_for_discord, cell_port_for_discord) =
        (config.auth_port, config.base_port, config.cell_port);

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

    if discord_enabled {
        cimmeria_discord::emit_server_startup(
            env!("CARGO_PKG_VERSION").to_string(),
            vec![
                format!("auth :{}", auth_port_for_discord),
                format!("base :{}", base_port_for_discord),
                format!("cell :{}", cell_port_for_discord),
                format!("admin :{}", admin_port),
            ],
        );
    }

    // Wait for Ctrl-C.
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl-C");

    tracing::info!("Shutting down…");
    tracing::trace!("Calling stop_all");
    orch.stop_all().await;
    tracing::trace!("stop_all complete");

    // Gate the drain on the runtime actually being enabled and the
    // ServerShutdown event being routable — `discord_runtime` is `Some`
    // even when the config file was missing (we hand back a disabled
    // runtime so the tracing layer + emit_* helpers no-op uniformly),
    // and we don't want to pay 1 s on every Ctrl-C just for that.
    if discord_enabled
        && discord_runtime
            .config
            .load()
            .should_post(cimmeria_discord::EventKind::ServerShutdown)
    {
        let uptime = server_start.elapsed().as_secs();
        cimmeria_discord::emit_server_shutdown("Ctrl-C", uptime);
        // Give the Discord sender task ~1 s to drain before the runtime
        // shuts down. Beyond that, drop on the floor — the user is
        // waiting for shutdown to complete.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    tracing::info!("Goodbye.");
    tracing::trace!(pid = std::process::id(), "Process exiting with code 0");
}

// ── Audit persistence ────────────────────────────────────────────────────────

/// Background task that persists [`LoginEvent`]s to the `login_audit` table.
///
/// Tolerates DB unavailability — logs a warning and keeps running.
async fn audit_writer_loop(pool: sqlx::PgPool, mut rx: broadcast::Receiver<LoginEvent>) {
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

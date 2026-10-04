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
//! - `CIMMERIA_LAB_START32` — the i686 `sgw-start32.exe` helper that
//!   injects the DLL (#985); default: beside this executable.
//! - `CIMMERIA_LAB_BRIDGE_BIND` / `_PORT`, `CIMMERIA_LAB_UPLOAD_ENDPOINT`
//!   — written into the session file the supervisor generates.
//! - `CIMMERIA_LAB_SERVER_URL`, `CIMMERIA_LAB_TELEMETRY` — where each
//!   launch mints its telemetry token (default: the client's login server)
//!   and whether a failed mint stops the launch (`supervisor::telemetry_session`).
//! - `CIMMERIA_LAB_INSTANCE` — names this supervisor's client (`p2`) so a
//!   second `cimmeria-lab` can drive a second client: its own session file,
//!   `lab-account.<name>.json`, logs and (with `_PORT`) bridge port.
//!   `CIMMERIA_LAB_MAX_CLIENTS` caps the clients (default 2). Unset = the
//!   single-client layout.
//! - `CIMMERIA_LAB_UAT_P2`, `CIMMERIA_LAB_UAT_P2_BRIDGE_PORT` — the second
//!   instance `lab_uat_run` drives for two-player rows (default `p2`, this
//!   port + 1); its account is `lab-account.<name>.json`.
//!
//! Modes:
//!
//! - default: one stdio MCP server per Claude session (back-compat).
//! - `--http <127.0.0.1:port> [--log-file <path>]`: the shared daemon
//!   ([`daemon`]), token-gated by `CIMMERIA_LAB_DAEMON_TOKEN`, logging to
//!   `%LOCALAPPDATA%\cimmeria-lab\labd.log`. `tools/lab/daemon.ps1` runs it
//!   as a per-user scheduled task.

use std::sync::Arc;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

mod client;
mod daemon;
mod server;
mod supervisor;
mod timeline;
mod uat;

use client::BridgeClient;
use daemon::config::Mode;
use server::LabServer;
use supervisor::{Supervisor, SupervisorConfig};

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

fn main() -> Result<()> {
    let mode = match daemon::config::parse_args(std::env::args().skip(1)) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("cimmeria-lab: {e}");
            eprintln!(
                "usage: cimmeria-lab [--http <loopback ip:port, e.g. {}> [--log-file <path>]]",
                daemon::config::DEFAULT_BIND
            );
            std::process::exit(daemon::EXIT_REFUSED);
        }
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    match mode {
        Mode::Stdio => rt.block_on(run_stdio()),
        Mode::Http { bind, log_file } => {
            let code = rt.block_on(run_http(bind, log_file));
            // Drop the runtime first so the log guard flushes, then exit with
            // the code the scheduled-task wrapper keys its restart on.
            drop(rt);
            std::process::exit(code)
        }
    }
}

/// Build the shared supervisor + MCP server from the environment.
fn build_server() -> Result<LabServer> {
    // A bad instance name must stop the server: silently becoming the
    // default instance would put two clients on one session file.
    if let Err(e) = supervisor::instance::from_env() {
        anyhow::bail!(e);
    }
    let config = SupervisorConfig::from_env();
    // The bridge address follows the configured port, so a second instance
    // needs only `CIMMERIA_LAB_BRIDGE_PORT`.
    let addr = std::env::var("CIMMERIA_LAB_BRIDGE")
        .unwrap_or_else(|_| format!("127.0.0.1:{}", config.port));
    let token = std::env::var("CIMMERIA_LAB_TOKEN").unwrap_or_default();
    if token.is_empty() {
        tracing::warn!(
            "CIMMERIA_LAB_TOKEN is unset; attaching to a pre-existing client will fail until lab_client_start mints its own token"
        );
    }
    tracing::info!(%addr, install_dir = ?config.install_dir, instance = ?config.instance,
        "cimmeria-lab supervisor starting");

    let bridge = Arc::new(BridgeClient::new(addr, token));
    let supervisor = Arc::new(Supervisor::new(bridge, config));
    Ok(LabServer::new(supervisor))
}

async fn run_stdio() -> Result<()> {
    // Logs go to stderr — stdout is the MCP transport.
    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let service = build_server()?
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("serve error: {e:?}"))?;

    service.waiting().await?;
    Ok(())
}

/// The shared daemon. Returns the process exit code.
async fn run_http(bind: String, log_file: Option<std::path::PathBuf>) -> i32 {
    use daemon::config::{self, DaemonConfig};
    use daemon::single_instance::{self, InstanceGuard};

    let state = config::state_dir();
    let log_path = log_file.unwrap_or_else(|| state.join("labd.log"));
    let writer = match daemon::log_file::RotatingFile::open(
        &log_path,
        daemon::log_file::MAX_BYTES,
        daemon::log_file::KEEP,
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("cimmeria-lab: cannot open {}: {e}", log_path.display());
            return daemon::EXIT_REFUSED;
        }
    };
    let (nb, _log_guard) = tracing_appender::non_blocking(writer);
    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .with_writer(nb)
        .with_ansi(false)
        .init();

    let (addr, token) = match config::evaluate(&bind, std::env::var(config::ENV_TOKEN).ok()) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(reason = %e, "lab daemon refused to start");
            eprintln!("cimmeria-lab: {e}");
            return daemon::EXIT_REFUSED;
        }
    };
    let cfg = DaemonConfig {
        bind: addr,
        token,
        log_file: log_path,
    };

    let pidfile = state.join("labd.pid");
    let mut guard = match InstanceGuard::acquire(single_instance::MUTEX_NAME) {
        Ok(g) => g,
        Err(e) => {
            let holder = single_instance::describe_holder(&pidfile);
            tracing::error!(reason = %e, %holder, "lab daemon already running; not starting");
            eprintln!("cimmeria-lab: {e} ({holder})");
            return daemon::EXIT_ALREADY_RUNNING;
        }
    };
    let listener = match tokio::net::TcpListener::bind(cfg.bind).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(addr = %cfg.bind, error = %e, "lab daemon cannot bind; is another one running?");
            eprintln!("cimmeria-lab: cannot bind {}: {e}", cfg.bind);
            return daemon::EXIT_ALREADY_RUNNING;
        }
    };
    if let Err(e) = guard.write_pidfile(&pidfile, &cfg.bind.to_string()) {
        tracing::warn!(error = %e, path = %pidfile.display(), "could not write the pidfile");
    }

    let server = match build_server() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "lab daemon refused to start");
            return daemon::EXIT_REFUSED;
        }
    };
    tracing::info!(addr = %cfg.bind, log = %cfg.log_file.display(), pid = std::process::id(),
        "lab daemon listening");
    let router = daemon::build_router(server, &cfg.token);
    match daemon::serve(listener, router).await {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!(error = %e, "lab daemon server error");
            1
        }
    }
}

//! PostgreSQL auto-start support for the orchestrator.
//!
//! On Windows dev boxes we ship a bundled PostgreSQL under
//! `external/postgresql_server/`. If it isn't already listening on the
//! configured port at server startup, we try to launch it via `pg_ctl`
//! before the orchestrator opens its connection pool.

use std::path::PathBuf;
use std::time::Duration;

/// Parse the `port=NNNN` value from a libpq-style connection string.
fn parse_pg_port(conn_str: &str) -> u16 {
    for part in conn_str.split_whitespace() {
        if let Some(val) = part.strip_prefix("port=") {
            if let Ok(p) = val.parse::<u16>() {
                return p;
            }
        }
    }
    5433 // project default
}

/// Parse the `host=...` value from a libpq-style connection string.
fn parse_pg_host(conn_str: &str) -> String {
    for part in conn_str.split_whitespace() {
        if let Some(val) = part.strip_prefix("host=") {
            return val.to_string();
        }
    }
    "localhost".to_string()
}

/// Try to find pg_ctl.exe by searching common project-relative paths.
fn find_pg_ctl() -> Option<PathBuf> {
    let candidates = [
        // Relative to CWD (typical: running from project root)
        PathBuf::from("external/postgresql_server/bin/pg_ctl.exe"),
        // One level up (if running from a subdirectory)
        PathBuf::from("../external/postgresql_server/bin/pg_ctl.exe"),
    ];

    for path in &candidates {
        if path.exists() {
            tracing::debug!(path = %path.display(), "Found pg_ctl");
            return Some(path.clone());
        }
    }

    // Try relative to the executable location
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let pg_ctl = exe_dir.join("external/postgresql_server/bin/pg_ctl.exe");
            if pg_ctl.exists() {
                tracing::debug!(path = %pg_ctl.display(), "Found pg_ctl relative to exe");
                return Some(pg_ctl);
            }
        }
    }

    None
}

/// Find the PostgreSQL data directory.
fn find_pg_data() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("server/pgdata"),
        PathBuf::from("../server/pgdata"),
    ];

    for path in &candidates {
        if path.join("PG_VERSION").exists() {
            tracing::debug!(path = %path.display(), "Found pgdata directory");
            return Some(path.clone());
        }
    }

    None
}

/// Check if PostgreSQL is reachable on the given host:port.
async fn pg_is_running(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    tokio::net::TcpStream::connect(&addr)
        .await
        .is_ok()
}

/// Ensure PostgreSQL is running before we try to connect. If it's not listening
/// on the configured port, attempt to start it using `pg_ctl` from the bundled
/// PostgreSQL installation at `external/postgresql_server/`.
pub(crate) async fn ensure_postgresql_running(conn_str: &str) {
    let host = parse_pg_host(conn_str);
    let port = parse_pg_port(conn_str);

    // Only auto-start for localhost connections
    if host != "localhost" && host != "127.0.0.1" && host != "::1" {
        tracing::debug!(host = %host, "PostgreSQL host is remote — skipping auto-start");
        return;
    }

    if pg_is_running(&host, port).await {
        tracing::info!(port, "PostgreSQL already running");
        return;
    }

    tracing::warn!(port, "PostgreSQL not responding — attempting auto-start");

    let pg_ctl = match find_pg_ctl() {
        Some(p) => p,
        None => {
            tracing::warn!(
                "pg_ctl.exe not found in external/postgresql_server/bin/ — \
                 cannot auto-start PostgreSQL. Start it manually or check your project layout."
            );
            return;
        }
    };

    let pg_data = match find_pg_data() {
        Some(p) => p,
        None => {
            tracing::warn!(
                "PostgreSQL data directory (server/pgdata/) not found — \
                 run `Initialize-CimmeriaDatabase` first to set up the database."
            );
            return;
        }
    };

    // Ensure log directory exists
    let log_dir = PathBuf::from("server/logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("postgresql.log");

    tracing::info!(
        pg_ctl = %pg_ctl.display(),
        data_dir = %pg_data.display(),
        port,
        "Starting PostgreSQL"
    );

    let result = std::process::Command::new(&pg_ctl)
        .arg("start")
        .arg("-D")
        .arg(&pg_data)
        .arg("-l")
        .arg(&log_file)
        .arg("-o")
        .arg(format!("-p {}", port))
        .arg("-w") // wait for startup to complete
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                tracing::info!(port, "PostgreSQL started successfully");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::warn!(
                    "pg_ctl start returned non-zero: stdout={}, stderr={}",
                    stdout.trim(),
                    stderr.trim()
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to run pg_ctl: {e}");
        }
    }

    // Wait for PostgreSQL to accept connections (up to 10 seconds)
    for i in 0..20 {
        if pg_is_running(&host, port).await {
            tracing::info!(port, attempts = i + 1, "PostgreSQL is accepting connections");
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::warn!(
        port,
        "PostgreSQL did not start accepting connections within 10 seconds. \
         Check server/logs/postgresql.log for details."
    );
}

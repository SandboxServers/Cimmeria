//! `cimmeria-server --check-db`: the container's database login check.
//!
//! Reads the connection string from `DB_URL` (never from the command line,
//! so the password stays out of argv and `ps`), connects with the server's
//! own parser, runs `SELECT 1`, and exits 0, or prints a sanitised reason
//! to stderr and exits 1. The timeout is `PGCONNECT_TIMEOUT` seconds
//! (default 5). The s6 run script and the image's HEALTHCHECK call it.

use std::time::Duration;

use cimmeria_common::ServerConfig;
use cimmeria_services::database_check::check_database;

/// The command-line flag that selects this mode.
pub(crate) const FLAG: &str = "--check-db";

/// Default seconds for the whole check.
const DEFAULT_TIMEOUT_SECS: u64 = 5;

/// Run the check and return the process exit code.
pub(crate) async fn run() -> i32 {
    let conn =
        std::env::var("DB_URL").unwrap_or_else(|_| ServerConfig::default().db_connection_string);
    let timeout = Duration::from_secs(timeout_secs(std::env::var("PGCONNECT_TIMEOUT").ok()));
    match check_database(&conn, timeout).await {
        Ok(()) => {
            println!("database check ok");
            0
        }
        Err(reason) => {
            eprintln!("database check failed: {reason}");
            1
        }
    }
}

/// `PGCONNECT_TIMEOUT` as whole seconds; anything missing, non-numeric or
/// zero falls back to the default.
fn timeout_secs(raw: Option<String>) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|&s| s > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_defaults_and_rejects_junk() {
        assert_eq!(timeout_secs(None), 5);
        assert_eq!(timeout_secs(Some("7".into())), 7);
        assert_eq!(timeout_secs(Some(" 9 ".into())), 9);
        for junk in ["", "0", "-3", "5s", "abc"] {
            assert_eq!(timeout_secs(Some(junk.into())), 5, "{junk:?}");
        }
    }
}

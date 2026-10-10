//! The orchestrator's startup database connection.
//!
//! A configured database is required: when it cannot be reached, or rejects
//! the configured credentials, the server refuses to start rather than run
//! without a pool. Only an empty connection string runs without one.

use std::time::Duration;

use cimmeria_common::ServerConfig;

use crate::database::DatabasePool;
use crate::orchestrator::OrchestratorError;
use crate::orchestrator_postgres::ensure_postgresql_running;

/// Default for the orchestrator's startup database wait. sqlx retries a
/// refused connection until its own acquire timeout (30 s); this matches it.
pub(crate) const DB_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Connect to the database `config` names, or return `Ok(None)` when none is
/// configured (an empty connection string, a developer-only setup).
///
/// A configured database that refuses the connection, rejects the
/// credentials, or does not answer within `timeout` is an error: the caller
/// must not start serving logins without it.
pub(crate) async fn connect_database(
    config: &ServerConfig,
    timeout: Duration,
) -> Result<Option<DatabasePool>, OrchestratorError> {
    if !config.database_configured() {
        tracing::warn!(
            reason = "no_database_configured",
            developer_mode = config.developer_mode,
            "No database configured (DB_URL is empty): starting without a database. \
             Logins are refused unless developer mode is on, which accepts any \
             credential as access level 99. Developer use only."
        );
        return Ok(None);
    }

    // Ensure PostgreSQL is running (auto-start the bundled one if possible).
    ensure_postgresql_running(&config.db_connection_string).await;

    // Never log the connection string: it carries the database password.
    tracing::trace!("Connecting to database");
    let failure =
        match tokio::time::timeout(timeout, DatabasePool::connect(&config.db_connection_string))
            .await
        {
            Ok(Ok(pool)) => {
                tracing::info!("Database connected");
                return Ok(Some(pool));
            }
            Ok(Err(e)) => e.to_string(),
            Err(_) => format!("no connection within {:?}", timeout),
        };
    tracing::error!(
        reason = "database_connect_failed",
        error = %failure,
        "Database connection failed; refusing to start. Check DB_URL (host, port, \
         user, password, database name), or set DB_URL empty to run without a database."
    );
    Err(OrchestratorError::DatabaseFailed(failure))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::Orchestrator;

    /// A config whose every listener is on loopback and an ephemeral port,
    /// with `db_connection_string` set to `db_url` and developer mode on,
    /// so the tests also cover the developer configuration.
    fn fail_closed_config(db_url: String) -> ServerConfig {
        let port = || {
            std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port()
        };
        ServerConfig {
            logon_port: port(),
            auth_tls_port: port(),
            base_port: port(),
            cell_port: port(),
            admin_port: port(),
            minigame_port: port(),
            db_connection_string: db_url,
            developer_mode: true,
            ..ServerConfig::loopback()
        }
    }

    /// Run `start_all` against `db_url` and return its result. The login
    /// port is returned too, so a test can check nothing was bound.
    async fn start_with_db(db_url: String) -> (Result<(), OrchestratorError>, u16) {
        let config = fail_closed_config(db_url);
        let logon_port = config.logon_port;
        let mut orch = Orchestrator::new(config);
        orch.set_db_connect_timeout(Duration::from_secs(3));
        (orch.start_all().await, logon_port)
    }

    /// **Fail-closed guard.** A configured database on a closed port must
    /// stop `start_all` with `DatabaseFailed` before the login listener
    /// binds, even with developer mode on. `127.0.0.1:1` is the address the
    /// live-DB URL guard reserves for a pool that never connects. Under WSL2
    /// mirrored networking a closed IPv4 loopback port can hang instead of
    /// refusing; the startup timeout turns that into the same refusal.
    #[tokio::test]
    async fn start_all_fails_when_configured_database_is_unreachable() {
        let (result, logon_port) =
            start_with_db("postgres://w-testing:w-testing@127.0.0.1:1/sgw".to_string()).await;
        match result {
            // A connection failure, not a URL the driver could not parse.
            Err(OrchestratorError::DatabaseFailed(msg)) => assert!(
                !msg.contains("configuration"),
                "expected a connection failure, got {msg}"
            ),
            other => {
                panic!("an unreachable configured database must refuse to start, got {other:?}")
            }
        }
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", logon_port))
                .await
                .is_err(),
            "the login listener must not be bound after a refused start"
        );
    }

    /// **Fail-closed guard, bad credentials.** The case the container's
    /// `pg_isready` gate cannot see: PostgreSQL is up but rejects the
    /// configured user or password. `start_all` must refuse to start.
    #[tokio::test]
    async fn start_all_fails_when_configured_database_rejects_credentials() {
        let port = crate::fake_postgres::spawn_rejecting_postgres(
            "28P01",
            "password authentication failed for user \"w-testing\"",
        )
        .await;
        let (result, logon_port) = start_with_db(format!(
            "host=127.0.0.1 port={port} user=w-testing password=wrong dbname=sgw"
        ))
        .await;
        match result {
            Err(OrchestratorError::DatabaseFailed(msg)) => assert!(
                msg.contains("password authentication failed"),
                "the refusal should carry the server's reason, got {msg}"
            ),
            other => panic!("rejected credentials must refuse to start, got {other:?}"),
        }
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", logon_port))
                .await
                .is_err(),
            "the login listener must not be bound after a refused start"
        );
    }

    /// An empty connection string is the one way to run without a database:
    /// `connect_database` returns no pool instead of an error.
    #[tokio::test]
    async fn connect_database_without_a_configured_database_returns_none() {
        let config = ServerConfig {
            db_connection_string: String::new(),
            ..ServerConfig::loopback()
        };
        let pool = connect_database(&config, Duration::from_secs(1))
            .await
            .expect("no configured database is not an error");
        assert!(pool.is_none());
    }
}

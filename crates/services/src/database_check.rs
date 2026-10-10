//! One-shot database login check, for the container's start gate and
//! healthcheck (`cimmeria-server --check-db`).
//!
//! It connects with the server's own connection string and parser (libpq
//! key-value or a `postgres://` URL, percent-encoding included), runs
//! `SELECT 1`, and reports a failure as a fixed, sanitised reason. The
//! reason never carries the connection string, the password or the
//! server's free-text error message, so it is safe to print in container
//! logs. The connection string is read by the caller from the environment,
//! never from the command line, so the password does not appear in argv.

use std::fmt;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

use crate::database::libpq_to_url;

/// Why a database check failed. `Display` is the sanitised reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbCheckFailure {
    /// The connection string is empty.
    NotConfigured,
    /// The connection string could not be parsed.
    InvalidConnectionString,
    /// The server rejected the user or password (SQLSTATE 28P01 / 28000).
    AuthenticationFailed,
    /// The named database does not exist (SQLSTATE 3D000).
    DatabaseMissing,
    /// The server is starting up or shutting down (SQLSTATE 57P03). sqlx
    /// retries transient codes like this one until the timeout, so it is
    /// usually reported as [`TimedOut`](Self::TimedOut).
    ServerNotReady,
    /// The server refused the login with another SQLSTATE.
    Refused { sqlstate: String },
    /// The TCP connection was refused.
    ConnectionRefused,
    /// No connection, or no answer, within the timeout.
    TimedOut { secs: u64 },
    /// Any other failure (network, TLS, protocol), by kind only.
    Other { kind: &'static str },
}

impl fmt::Display for DbCheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => f.write_str("no database is configured (DB_URL is empty)"),
            Self::InvalidConnectionString => f.write_str("DB_URL could not be parsed"),
            Self::AuthenticationFailed => {
                f.write_str("the server rejected the user or password (SQLSTATE 28P01/28000)")
            }
            Self::DatabaseMissing => f.write_str("the database does not exist (SQLSTATE 3D000)"),
            Self::ServerNotReady => {
                f.write_str("the server is not accepting logins yet (SQLSTATE 57P03)")
            }
            Self::Refused { sqlstate } => {
                write!(f, "the server refused the login (SQLSTATE {sqlstate})")
            }
            Self::ConnectionRefused => f.write_str("the connection was refused"),
            Self::TimedOut { secs } => {
                write!(f, "no connection within {secs} s (refused or unreachable)")
            }
            Self::Other { kind } => write!(f, "connection failed ({kind})"),
        }
    }
}

impl std::error::Error for DbCheckFailure {}

/// Connect with `conn_str`, run `SELECT 1`, and close. `timeout` bounds the
/// whole check.
pub async fn check_database(conn_str: &str, timeout: Duration) -> Result<(), DbCheckFailure> {
    if conn_str.trim().is_empty() {
        return Err(DbCheckFailure::NotConfigured);
    }
    let url = if conn_str.starts_with("postgres://") || conn_str.starts_with("postgresql://") {
        conn_str.to_string()
    } else {
        libpq_to_url(conn_str)
    };
    let secs = timeout.as_secs().max(1);
    let attempt = async {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(timeout)
            .connect(&url)
            .await
            .map_err(|e| classify(&e, secs))?;
        let result = sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&pool)
            .await
            .map(|_| ())
            .map_err(|e| classify(&e, secs));
        pool.close().await;
        result
    };
    match tokio::time::timeout(timeout, attempt).await {
        Ok(result) => result,
        Err(_) => Err(DbCheckFailure::TimedOut { secs }),
    }
}

/// Map a sqlx error to a sanitised reason. Only the error's kind and
/// SQLSTATE are kept; its text can echo parts of the connection string.
fn classify(err: &sqlx::Error, secs: u64) -> DbCheckFailure {
    match err {
        sqlx::Error::Configuration(_) => DbCheckFailure::InvalidConnectionString,
        sqlx::Error::Database(db) => match db.code().as_deref() {
            Some("28P01") | Some("28000") => DbCheckFailure::AuthenticationFailed,
            Some("3D000") => DbCheckFailure::DatabaseMissing,
            Some("57P03") => DbCheckFailure::ServerNotReady,
            Some(code) if code.len() == 5 && code.bytes().all(|b| b.is_ascii_alphanumeric()) => {
                DbCheckFailure::Refused {
                    sqlstate: code.to_string(),
                }
            }
            _ => DbCheckFailure::Other {
                kind: "database error",
            },
        },
        sqlx::Error::Io(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
            DbCheckFailure::ConnectionRefused
        }
        sqlx::Error::Io(_) => DbCheckFailure::Other {
            kind: "network error",
        },
        sqlx::Error::Tls(_) => DbCheckFailure::Other { kind: "TLS error" },
        sqlx::Error::PoolTimedOut => DbCheckFailure::TimedOut { secs },
        sqlx::Error::Protocol(_) => DbCheckFailure::Other {
            kind: "protocol error",
        },
        _ => DbCheckFailure::Other {
            kind: "unexpected error",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake_postgres::spawn_rejecting_postgres;

    const SECRET: &str = "s3cretPW";

    async fn check_against(sqlstate: &'static str, message: &'static str) -> DbCheckFailure {
        let port = spawn_rejecting_postgres(sqlstate, message).await;
        let conn =
            format!("host=127.0.0.1 port={port} user=w-testing password={SECRET} dbname=sgw");
        check_database(&conn, Duration::from_secs(3))
            .await
            .expect_err("a rejected login must fail the check")
    }

    /// A rejected password is reported as an authentication failure, and the
    /// reason carries neither the password nor the server's free text.
    #[tokio::test]
    async fn rejected_password_is_reported_without_the_password() {
        let failure = check_against(
            "28P01",
            "password authentication failed for user \"w-testing\" s3cretPW",
        )
        .await;
        assert_eq!(failure, DbCheckFailure::AuthenticationFailed);
        let text = failure.to_string();
        assert!(!text.contains(SECRET), "reason leaked the password: {text}");
        assert!(
            !text.contains("w-testing"),
            "reason leaked the server text: {text}"
        );
    }

    #[tokio::test]
    async fn missing_database_is_reported() {
        let failure = check_against("3D000", "database \"sgw\" does not exist").await;
        assert_eq!(failure, DbCheckFailure::DatabaseMissing);
    }

    #[tokio::test]
    async fn other_sqlstate_is_reported_by_code_only() {
        let failure = check_against("42501", "permission denied for s3cretPW").await;
        assert_eq!(
            failure,
            DbCheckFailure::Refused {
                sqlstate: "42501".to_string()
            }
        );
        assert!(!failure.to_string().contains(SECRET));
    }

    #[tokio::test]
    async fn empty_connection_string_is_not_configured() {
        assert_eq!(
            check_database("  ", Duration::from_secs(1)).await,
            Err(DbCheckFailure::NotConfigured)
        );
    }

    /// An unparseable string names no fragment of itself in the reason.
    #[tokio::test]
    async fn unparseable_connection_string_is_reported_without_echo() {
        let conn =
            format!("host=127.0.0.1 port=notaport user=w-testing password={SECRET} dbname=sgw");
        let failure = check_database(&conn, Duration::from_secs(1))
            .await
            .expect_err("an invalid port must fail");
        assert_eq!(failure, DbCheckFailure::InvalidConnectionString);
        assert!(!failure.to_string().contains(SECRET));
    }

    /// A pool that never connects (`127.0.0.1:1`, reserved by the live-DB
    /// URL guard) fails within the timeout as refused or timed out.
    #[tokio::test]
    async fn unreachable_server_fails_within_the_timeout() {
        let started = std::time::Instant::now();
        let failure = check_database(
            "host=127.0.0.1 port=1 user=w-testing password=w-testing dbname=sgw",
            Duration::from_secs(2),
        )
        .await
        .expect_err("nothing listens on port 1");
        assert!(
            matches!(
                failure,
                DbCheckFailure::ConnectionRefused | DbCheckFailure::TimedOut { .. }
            ),
            "got {failure:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}

//! Database connection pool.
//!
//! Wraps sqlx's PostgreSQL connection pool with health checking and
//! configuration from the server config. Corresponds to the SOCI 3.2.1
//! session management in the original C++ codebase.

use sqlx::PgPool;

/// Errors specific to database operations.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Failed to connect to database: {0}")]
    ConnectionFailed(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Query error: {0}")]
    Query(#[from] sqlx::Error),
}

/// PostgreSQL connection pool for the Cimmeria server.
///
/// Manages a pool of database connections used by all three services
/// (Auth, Base, Cell) for account queries, entity persistence, and
/// content loading. Replaces the SOCI 3.2.1 `soci::session` / connection
/// pool from the original C++ codebase.
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// Connect to the database using the given connection string.
    ///
    /// The connection string format matches the PostgreSQL libpq format
    /// used in the original `BaseService.config`:
    /// `host=localhost port=5433 user=w-testing password=w-testing dbname=sgw`
    ///
    /// Internally this converts to a `postgres://` URL for sqlx.
    pub async fn connect(connection_string: &str) -> Result<Self, DatabaseError> {
        tracing::info!("Connecting to database");

        // Convert libpq-style connection string to URL format if needed
        let url = if connection_string.starts_with("postgres://")
            || connection_string.starts_with("postgresql://")
        {
            connection_string.to_string()
        } else {
            libpq_to_url(connection_string)
        };

        let pool = PgPool::connect(&url)
            .await
            .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        tracing::info!("Database connection pool established");
        Ok(Self { pool })
    }

    /// Get a reference to the underlying connection pool.
    ///
    /// Used by services to execute queries directly.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check whether the database is reachable.
    ///
    /// Executes a simple `SELECT 1` query to verify connectivity.
    pub async fn health_check(&self) -> bool {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }
}

/// Convert a libpq-style connection string to a postgres:// URL.
///
/// Parses key=value pairs (host, port, user, password, dbname) from the
/// format used in the original C++ config files.
fn libpq_to_url(conn_str: &str) -> String {
    let mut host = "localhost";
    let mut port = "5432";
    let mut user = "postgres";
    let mut password = "";
    let mut dbname = "postgres";

    for part in conn_str.split_whitespace() {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "host" => host = value,
                "port" => port = value,
                "user" => user = value,
                "password" => password = value,
                "dbname" => dbname = value,
                _ => {}
            }
        }
    }

    if password.is_empty() {
        format!("postgres://{user}@{host}:{port}/{dbname}")
    } else {
        format!("postgres://{user}:{password}@{host}:{port}/{dbname}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libpq_to_url_full() {
        let url = libpq_to_url("host=localhost port=5433 user=w-testing password=w-testing dbname=sgw");
        assert_eq!(url, "postgres://w-testing:w-testing@localhost:5433/sgw");
    }

    #[test]
    fn libpq_to_url_no_password() {
        let url = libpq_to_url("host=db.local port=5432 user=admin dbname=mydb");
        assert_eq!(url, "postgres://admin@db.local:5432/mydb");
    }

    #[test]
    fn libpq_to_url_defaults() {
        let url = libpq_to_url("");
        assert_eq!(url, "postgres://postgres@localhost:5432/postgres");
    }
}

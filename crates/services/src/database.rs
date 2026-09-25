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

// ── Read-only lab query ───────────────────────────────────────────────────────
//
// Backs the live-research-lab MCP `server_db_query` tool (issue #687). Lives
// here, next to the pool, so its live-DB regression guards run under the
// established `-p cimmeria-services --lib` live-DB job.

/// Default cap on rows returned by the lab `server_db_query` tool.
pub const LAB_QUERY_ROW_CAP: usize = 500;

/// Result of a [`read_only_query`].
#[derive(Debug)]
pub struct ReadOnlyQueryResult {
    /// Each row projected to a single JSON object (via `to_jsonb`).
    pub rows: Vec<serde_json::Value>,
    /// `true` if the query produced more than the row cap (extra rows dropped).
    pub truncated: bool,
}

/// Wrap a single user statement into a capped, JSON-projecting SELECT.
///
/// Pure, so the single-statement + wrapping rules can be unit-tested without a
/// database. The wrap is the first of three write-rejection layers (see
/// [`read_only_query`]): a data-modifying statement is a syntax error as a
/// FROM sub-select, so it never reaches execution.
pub fn prepare_read_only_query(sql: &str, row_cap: usize) -> Result<String, String> {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return Err("empty query".to_string());
    }
    // A remaining `;` means more than one statement was submitted.
    if trimmed.contains(';') {
        return Err("only a single statement is permitted".to_string());
    }
    // `to_jsonb(t)` turns each result row into one JSON value we can decode
    // generically; the FROM sub-select forces a read query; the outer LIMIT
    // caps the result. `row_cap + 1` lets the caller detect truncation.
    Ok(format!(
        "SELECT to_jsonb(t) AS lab_row FROM ({trimmed}) AS t LIMIT {}",
        row_cap.saturating_add(1)
    ))
}

/// Execute one user statement as a capped, read-only JSON query.
///
/// Three write-rejection layers: (1) [`prepare_read_only_query`] wraps it as a
/// FROM sub-select, so any write is a syntax error; (2) it runs inside a
/// `READ ONLY` transaction, so even a CTE-hidden write is refused by Postgres;
/// (3) the transaction is always rolled back, so nothing can persist. The
/// result is capped at `row_cap`.
pub async fn read_only_query(
    pool: &PgPool,
    sql: &str,
    row_cap: usize,
) -> Result<ReadOnlyQueryResult, String> {
    let wrapped = prepare_read_only_query(sql, row_cap)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("failed to open transaction: {e}"))?;
    // Must be the first statement in the transaction to apply to it.
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("failed to enter read-only mode: {e}"))?;

    // `query_as` requires a `&'static str` unless the caller asserts the SQL
    // is safe. The wrapped string is not static (it embeds the cap + the user
    // statement), and it IS audited: `prepare_read_only_query` forbids
    // multiple statements and wraps the user text as a FROM sub-select, and the
    // enclosing transaction is `READ ONLY`. `AssertSqlSafe` records that
    // review at the call site.
    let rows: Vec<(serde_json::Value,)> = sqlx::query_as(sqlx::AssertSqlSafe(wrapped))
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    // Explicit rollback (a dropped tx would also roll back — be explicit).
    let _ = tx.rollback().await;

    let truncated = rows.len() > row_cap;
    let rows: Vec<serde_json::Value> = rows.into_iter().take(row_cap).map(|(v,)| v).collect();
    Ok(ReadOnlyQueryResult { rows, truncated })
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
        let url =
            libpq_to_url("host=localhost port=5433 user=w-testing password=w-testing dbname=sgw");
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

    // ── prepare_read_only_query (pure) ─────────────────────────────────────

    #[test]
    fn prepare_rejects_empty() {
        assert!(prepare_read_only_query("   ", 500).is_err());
        assert!(prepare_read_only_query(";", 500).is_err());
    }

    #[test]
    fn prepare_rejects_multiple_statements() {
        // Embedded `;` (not just a trailing one) means two statements.
        let err = prepare_read_only_query("SELECT 1; DROP TABLE account", 500)
            .expect_err("multi-statement must be rejected");
        assert!(err.contains("single statement"));
    }

    #[test]
    fn prepare_strips_one_trailing_semicolon_and_wraps_with_cap() {
        let wrapped = prepare_read_only_query("SELECT 1 AS n;", 500).unwrap();
        // Wrapped as a FROM sub-select with to_jsonb projection + LIMIT cap+1.
        assert_eq!(
            wrapped,
            "SELECT to_jsonb(t) AS lab_row FROM (SELECT 1 AS n) AS t LIMIT 501"
        );
    }

    // ── read_only_query (live-DB) ──────────────────────────────────────────

    use crate::test_support::require_db_or_skip;

    /// A write statement must be rejected — `server_db_query` is read-only.
    /// Regression shape: drop the sub-select wrap AND the READ ONLY tx and a
    /// `DELETE`/`UPDATE` would execute against the live DB.
    #[tokio::test]
    async fn read_only_query_rejects_write() {
        let pool = require_db_or_skip!();
        // WHERE 1=0 so that even if the guard failed, no row would change —
        // the assertion is that the call itself is refused.
        let result = read_only_query(&pool, "DELETE FROM account WHERE 1=0", 500).await;
        assert!(
            result.is_err(),
            "a write statement must be rejected, got: {result:?}"
        );
    }

    /// A CTE-hidden write (data-modifying `WITH`) must also be rejected — the
    /// READ ONLY transaction is the backstop for anything the sub-select wrap
    /// doesn't catch as a syntax error.
    #[tokio::test]
    async fn read_only_query_rejects_cte_write() {
        let pool = require_db_or_skip!();
        let result = read_only_query(
            &pool,
            "WITH d AS (DELETE FROM account WHERE 1=0 RETURNING 1) SELECT * FROM d",
            500,
        )
        .await;
        assert!(
            result.is_err(),
            "a data-modifying CTE must be rejected, got: {result:?}"
        );
    }

    /// The row cap is enforced: a query producing more than the cap returns
    /// exactly `row_cap` rows and flags truncation. `generate_series` needs no
    /// seed data, so this guards the cap independently of the schema.
    #[tokio::test]
    async fn read_only_query_enforces_row_cap() {
        let pool = require_db_or_skip!();
        let result = read_only_query(&pool, "SELECT generate_series(1, 600) AS n", 500)
            .await
            .expect("a read query must succeed");
        assert_eq!(result.rows.len(), 500, "must cap at exactly row_cap rows");
        assert!(
            result.truncated,
            "producing more than the cap must set truncated=true"
        );
    }

    /// Control: a query under the cap returns all its rows and is not
    /// truncated — proves the cap test isn't passing vacuously.
    #[tokio::test]
    async fn read_only_query_under_cap_is_not_truncated() {
        let pool = require_db_or_skip!();
        let result = read_only_query(&pool, "SELECT generate_series(1, 3) AS n", 500)
            .await
            .expect("a read query must succeed");
        assert_eq!(result.rows.len(), 3);
        assert!(!result.truncated);
    }
}

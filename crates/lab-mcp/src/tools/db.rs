//! `server_db_query` tool logic — one read-only SQL statement, row-capped.
//!
//! The actual safety-critical execution (single-statement wrap + `READ ONLY`
//! transaction + row cap) lives in `cimmeria_services::database` so its
//! regression guards run under the established live-DB CI job. This module is
//! the thin tool-shaped wrapper over it.

use serde_json::{json, Value};

use cimmeria_services::database::{read_only_query, LAB_QUERY_ROW_CAP};

use crate::state::LabState;

/// Execute one read-only statement and return the rows as JSON, capped at
/// `LAB_QUERY_ROW_CAP`. Writes are rejected (see `read_only_query`).
pub async fn db_query(state: &LabState, sql: String) -> Result<Value, String> {
    let Some(pool) = state.db_pool().await else {
        return Err("database not connected".to_string());
    };
    let result = read_only_query(&pool, &sql, LAB_QUERY_ROW_CAP).await?;
    Ok(json!({
        "row_count": result.rows.len(),
        "truncated": result.truncated,
        "row_cap": LAB_QUERY_ROW_CAP,
        "rows": result.rows,
    }))
}

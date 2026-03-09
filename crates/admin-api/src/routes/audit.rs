//! Login audit REST endpoints.
//!
//! Provides paginated access to login history from the `login_audit` table.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use cimmeria_services::orchestrator::Orchestrator;

/// Query parameters for `GET /api/audit/logins`.
#[derive(Deserialize)]
pub struct LoginAuditQuery {
    /// Max results per page (default 50, max 500).
    pub limit: Option<i64>,
    /// Cursor: return events before this ISO 8601 timestamp.
    pub before: Option<String>,
    /// Filter by account name.
    pub account_name: Option<String>,
    /// Filter by outcome (e.g. "success", "invalid_credentials").
    pub outcome: Option<String>,
}

#[derive(Serialize)]
struct LoginAuditRow {
    id: i64,
    event_time: String,
    account_name: String,
    account_id: Option<i32>,
    ip_address: String,
    phase: String,
    outcome: String,
    shard: Option<String>,
    detail: Option<String>,
}

#[derive(Serialize)]
struct LoginAuditResponse {
    status: String,
    events: Vec<LoginAuditRow>,
    count: usize,
    has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new().route("/logins", get(list_login_events))
}

/// `GET /api/audit/logins` — paginated login history with optional filters.
async fn list_login_events(
    State(orchestrator): State<Arc<Orchestrator>>,
    Query(params): Query<LoginAuditQuery>,
) -> impl IntoResponse {
    let state = orchestrator.state();
    let state = state.read().await;

    let pool = match &state.db {
        Some(db) => db.pool(),
        None => {
            return Json(LoginAuditResponse {
                status: "unavailable".into(),
                events: vec![],
                count: 0,
                has_more: false,
                reason: Some("Database not connected".into()),
            });
        }
    };

    let limit = params.limit.unwrap_or(50).min(500).max(1);
    let fetch_limit = limit + 1;

    // Build query with dynamic WHERE clauses.
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_idx: usize = 0;

    if params.before.is_some() {
        bind_idx += 1;
        conditions.push(format!("event_time < ${bind_idx}::TIMESTAMPTZ"));
    }
    if params.account_name.is_some() {
        bind_idx += 1;
        conditions.push(format!("account_name = ${bind_idx}"));
    }
    if params.outcome.is_some() {
        bind_idx += 1;
        conditions.push(format!("outcome = ${bind_idx}"));
    }
    bind_idx += 1;
    let limit_param = bind_idx;

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, TO_CHAR(event_time, 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"') AS event_time, \
         account_name, account_id, ip_address::TEXT, phase, outcome, shard, detail \
         FROM login_audit \
         {where_clause} \
         ORDER BY event_time DESC LIMIT ${limit_param}"
    );

    let mut query = sqlx::query_as::<_, AuditRow>(&sql);

    // Bind parameters in the same order as the placeholders.
    if let Some(ref before) = params.before {
        query = query.bind(before);
    }
    if let Some(ref account_name) = params.account_name {
        query = query.bind(account_name);
    }
    if let Some(ref outcome) = params.outcome {
        query = query.bind(outcome);
    }
    query = query.bind(fetch_limit);

    match query.fetch_all(pool).await {
        Ok(mut rows) => {
            let has_more = rows.len() as i64 > limit;
            rows.truncate(limit as usize);
            let count = rows.len();
            let events: Vec<LoginAuditRow> = rows
                .into_iter()
                .map(|r| LoginAuditRow {
                    id: r.id,
                    event_time: r.event_time,
                    account_name: r.account_name,
                    account_id: r.account_id,
                    ip_address: r.ip_address,
                    phase: r.phase,
                    outcome: r.outcome,
                    shard: r.shard,
                    detail: r.detail,
                })
                .collect();
            Json(LoginAuditResponse {
                status: "ok".into(),
                events,
                count,
                has_more,
                reason: None,
            })
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to query login_audit");
            Json(LoginAuditResponse {
                status: "unavailable".into(),
                events: vec![],
                count: 0,
                has_more: false,
                reason: Some(format!("Query failed: {e}")),
            })
        }
    }
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: i64,
    event_time: String,
    account_name: String,
    account_id: Option<i32>,
    ip_address: String,
    phase: String,
    outcome: String,
    shard: Option<String>,
    detail: Option<String>,
}

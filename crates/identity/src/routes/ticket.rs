//! The launch-ticket routes (contract C1 and C6): `POST /launch-ticket` mints
//! the ticket the game client logs in with, `heartbeat` keeps it alive and
//! `end` retires it.
//!
//! A ticket is never logged or traced: the spans carry the account id and the
//! outcome only, and the mint answer is `Cache-Control: no-store`.

use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use tracing::Instrument;

use crate::limits::Bucket;
use crate::repo::account;
use crate::routes::account::account_name;
use crate::routes::bearer::Bearer;
use crate::routes::ctx::{client_ip, RouteState};
use crate::routes::error::ApiError;
use crate::routes::signin::record_outcome;
use crate::stores::AccessGrant;
use crate::telemetry;

/// The shortest and longest `launch_id` (C6).
const LAUNCH_ID_LEN: std::ops::RangeInclusive<usize> = 16..=64;

/// The body of the three launch-ticket routes (C6). The field is optional here
/// so that a missing one is our `400 invalid_request`.
#[derive(Deserialize)]
pub struct LaunchRequest {
    launch_id: Option<String>,
}

/// Whether `raw` is a `launch_id`: 16 to 64 characters of `[A-Za-z0-9-]`.
pub(super) fn valid_launch_id(raw: &str) -> bool {
    LAUNCH_ID_LEN.contains(&raw.len())
        && raw
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// The validated `launch_id` of a request body, or `400 invalid_request`.
fn launch_id_of(body: Result<Json<LaunchRequest>, JsonRejection>) -> Result<String, ApiError> {
    body.ok()
        .and_then(|Json(request)| request.launch_id)
        .filter(|id| valid_launch_id(id))
        .ok_or(ApiError::InvalidRequest)
}

/// `POST /launch-ticket`: mints the ticket for the signed-in account and the
/// launch (C1, C6). The span is `auth.launch_ticket.mint` with no ticket field.
pub async fn mint(
    State(state): State<RouteState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Bearer(grant): Bearer,
    body: Result<Json<LaunchRequest>, JsonRejection>,
) -> Response {
    let span = telemetry::launch_ticket_mint(grant.account_id);
    let started = Instant::now();
    let ip = client_ip(ConnectInfo(peer));
    let mut response = match mint_ticket(&state, &ip, &grant, body)
        .instrument(span.clone())
        .await
    {
        Ok(response) => response,
        Err(error) => error.into_response(),
    };
    record_outcome(&span, started, &response);
    // The ticket is a credential: no cache may keep the answer.
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The work of `mint`. The rate limit runs before anything is minted.
async fn mint_ticket(
    state: &RouteState,
    ip: &str,
    grant: &AccessGrant,
    body: Result<Json<LaunchRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let now = Instant::now();
    let launch_id = launch_id_of(body)?;
    state.ctx.limit(Bucket::Token, ip, now)?;
    let pool = state.deps.db.as_deref().ok_or(ApiError::Unavailable)?;
    let account_id = grant.account_id;
    let facts = account::facts(pool, account_id)
        .await
        .map_err(ApiError::from_repo)?
        .ok_or(ApiError::InvalidToken)?;
    if !facts.enabled {
        return Err(ApiError::AccountDisabled);
    }
    let name = account_name(pool, account_id)
        .await
        .map_err(ApiError::from_repo)?
        .ok_or(ApiError::InvalidToken)?;
    let minted = state
        .deps
        .launch_tickets
        .mint(account_id, &name, &launch_id, now);
    Ok(Json(json!({
        "account_name": minted.account_name,
        "ticket": minted.ticket,
        "expires_in": minted.expires_in_secs,
    }))
    .into_response())
}

/// `POST /launch-ticket/heartbeat`: keeps the launch's ticket alive. `404` when
/// no live ticket of this account holds `launch_id` (C6).
pub async fn heartbeat(
    State(state): State<RouteState>,
    Bearer(grant): Bearer,
    body: Result<Json<LaunchRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let launch_id = launch_id_of(body)?;
    let alive = state
        .deps
        .launch_tickets
        .heartbeat(grant.account_id, &launch_id, Instant::now());
    if alive {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(ApiError::NotFound)
    }
}

/// `POST /launch-ticket/end`: retires the launch's ticket. Always `204`, also
/// when the ticket is already gone (C6). The span is `auth.launch_ticket.end`.
pub async fn end(
    State(state): State<RouteState>,
    Bearer(grant): Bearer,
    body: Result<Json<LaunchRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let launch_id = launch_id_of(body)?;
    let span = telemetry::launch_ticket_end(grant.account_id);
    span.record("outcome", "ok");
    let _entered = span.enter();
    state.deps.launch_tickets.end(grant.account_id, &launch_id);
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[cfg(test)]
mod tests {
    use super::valid_launch_id;

    #[test]
    fn launch_id_length_bounds() {
        assert!(!valid_launch_id(&"a".repeat(15)));
        assert!(valid_launch_id(&"a".repeat(16)));
        assert!(valid_launch_id(&"a".repeat(64)));
        assert!(!valid_launch_id(&"a".repeat(65)));
    }

    #[test]
    fn launch_id_alphabet() {
        assert!(valid_launch_id("Abc-123-xyz-9876-AB"));
        assert!(!valid_launch_id("abc_123_xyz_9876_ab"));
        assert!(!valid_launch_id("abc 123 xyz 9876 ab"));
        assert!(!valid_launch_id("abc.123.xyz.9876.ab"));
    }
}

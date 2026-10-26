//! The link routes of the sign-in flow (contract C6, C10 and C15).
//!
//! `POST /link/new` creates a passwordless account for the verified identity
//! behind a link grant. `POST /link/existing` links that identity to an account
//! the player names, by password or by a GM link code. Either ends in
//! [`signed_in`]. The link clears the account's password in the same
//! transaction (C15 a and b), so an account with an identity never holds one.

use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use cimmeria_auth::auth::{verify_account_password, PasswordCheck};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::Instrument;

use crate::limits::{old_enough, Bucket};
use crate::link;
use crate::names;
use crate::repo::event::{self, EventKind};
use crate::repo::{account, identity};
use crate::routes::ctx::{client_ip, note, RouteState};
use crate::routes::error::ApiError;
use crate::routes::token::signed_in;
use crate::telemetry;

/// The longest password the link form checks, in bytes. The check runs
/// before any hash is computed, so an oversized body costs nothing more.
pub const MAX_PASSWORD_BYTES: usize = 256;

/// The `POST /link/new` body (C6). Optional here so that a missing field is our
/// `400 invalid_request`.
#[derive(Deserialize)]
pub struct NewRequest {
    link_grant: Option<String>,
}

/// The `POST /link/existing` body (C6). The password form sends
/// `account_name` and `password`; the link-code form sends `link_code`.
/// `password_removal_ack` must be `true` (C15).
#[derive(Deserialize)]
pub struct ExistingRequest {
    link_grant: Option<String>,
    account_name: Option<String>,
    password: Option<String>,
    link_code: Option<String>,
    password_removal_ack: Option<bool>,
}

/// `POST /link/new`: creates an account for the link grant's identity and
/// signs it in (C6). Sign-up is open only when the config allows it.
pub async fn new(
    State(state): State<RouteState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    body: Result<Json<NewRequest>, JsonRejection>,
) -> Response {
    let span = telemetry::idp_link();
    let started = Instant::now();
    let result = create(&state, peer, body).instrument(span.clone()).await;
    answered(&span, started, result)
}

/// `POST /link/existing`: links the grant's identity to an existing account,
/// after the password or the link code proves the account is the player's.
pub async fn existing(
    State(state): State<RouteState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    body: Result<Json<ExistingRequest>, JsonRejection>,
) -> Response {
    let span = telemetry::idp_link();
    let started = Instant::now();
    let result = link_existing(&state, peer, body)
        .instrument(span.clone())
        .await;
    answered(&span, started, result)
}

/// The body of [`new`]: the grant, the sign-up gates, then the account.
async fn create(
    state: &RouteState,
    peer: SocketAddr,
    body: Result<Json<NewRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let now = Instant::now();
    let fence = state.ctx.stores.access.fence();
    let Json(request) = body.map_err(|_| ApiError::InvalidRequest)?;
    let grant = request.link_grant.ok_or(ApiError::InvalidRequest)?;
    // Peek, then take only once the sign-up checks pass, so a refused sign-up
    // keeps the grant and the provider sign-in is not repeated (C7).
    let peeked = state
        .ctx
        .stores
        .grants
        .peek(&grant, now)
        .ok_or(ApiError::InvalidGrant)?;
    if !state.ctx.cfg.signup_open {
        return Err(ApiError::SignupClosed);
    }
    // Both sign-up limits count before the account is created (C7).
    let key = client_ip(ConnectInfo(peer));
    state.ctx.limit(Bucket::SignupIp, &key, now)?;
    state.ctx.limit(Bucket::SignupGlobal, "", now)?;
    let min_days = state.ctx.cfg.limits.min_provider_account_age_days;
    let now_ms = Utc::now().timestamp_millis();
    if !old_enough(peeked.provider_account_created_ms, now_ms, min_days) {
        return Err(ApiError::SignupClosed);
    }

    let pool = db_pool(state)?;
    let verified = state
        .ctx
        .stores
        .grants
        .take(&grant, now)
        .ok_or(ApiError::InvalidGrant)?;
    let base = names::base_from_display(&verified.display_name);
    let candidates = names::candidates(&base, &mut rand::rng());
    let (account_id, _name) = account::create(pool, &candidates)
        .await
        .map_err(ApiError::from_repo)?;
    let store_email = state.trusts_email(&verified) && verified.email_verified;
    let linked = match identity::insert(pool, account_id, &verified, store_email).await {
        Ok(linked) => linked,
        Err(error) => {
            // No identity row: drop the account just made so no orphan keeps the name.
            let _ = account::remove_unlinked(pool, account_id).await;
            return Err(ApiError::from_repo(error));
        }
    };
    let provider = verified.provider.as_str();
    event::record(
        pool,
        account_id,
        Some(linked.identity_id),
        EventKind::Signup,
        Some(provider),
    )
    .await
    .map_err(ApiError::from_repo)?;
    note("signup");
    signed_in(state, account_id, None, fence, true, now).await
}

/// The body of [`existing`]: the ack, the limits, the credential check, then the
/// link itself. A refused link keeps its grant and password.
async fn link_existing(
    state: &RouteState,
    peer: SocketAddr,
    body: Result<Json<ExistingRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let now = Instant::now();
    let fence = state.ctx.stores.access.fence();
    let Json(request) = body.map_err(|_| ApiError::InvalidRequest)?;
    // The warning cannot be skipped: no limit is consumed and the grant stays.
    if request.password_removal_ack != Some(true) {
        return Err(ApiError::InvalidRequest);
    }
    let grant = request.link_grant.ok_or(ApiError::InvalidRequest)?;
    state
        .ctx
        .limit(Bucket::LinkExistingIp, &client_ip(ConnectInfo(peer)), now)?;
    let pool = db_pool(state)?;
    let peeked = state
        .ctx
        .stores
        .grants
        .peek(&grant, now)
        .ok_or(ApiError::InvalidGrant)?;

    let account_id = match request.link_code {
        Some(code) => {
            // Every redeem counts, right or wrong, against one server-wide cap.
            state.ctx.limit(Bucket::LinkCodeGlobal, "", now)?;
            state
                .ctx
                .link_codes
                .redeem(&code, now)
                .ok_or(ApiError::InvalidCredentials)?
        }
        None => {
            let (Some(name), Some(password)) = (request.account_name, request.password) else {
                return Err(ApiError::InvalidRequest);
            };
            // A name outside the account-name grammar cannot exist, so it is refused
            // before the limiter: its key is bounded by the grammar, not the body.
            if !names::is_valid_account_name(&name) {
                return Err(ApiError::InvalidRequest);
            }
            state
                .ctx
                .limit(Bucket::LinkExistingAccount, &name.to_ascii_lowercase(), now)?;
            check_password(pool, &name, &password).await?
        }
    };

    let facts = account::facts(pool, account_id)
        .await
        .map_err(ApiError::from_repo)?
        .ok_or(ApiError::NotLinkable)?;
    let provider = peeked.provider.as_str();
    if !facts.enabled {
        return Err(ApiError::AccountDisabled);
    }
    if !link::linkable(&facts) {
        event::record(
            pool,
            account_id,
            None,
            EventKind::LinkRefused,
            Some(provider),
        )
        .await
        .map_err(ApiError::from_repo)?;
        return Err(ApiError::NotLinkable);
    }

    let verified = state
        .ctx
        .stores
        .grants
        .take(&grant, now)
        .ok_or(ApiError::InvalidGrant)?;
    let store_email = state.trusts_email(&verified) && verified.email_verified;
    let linked = identity::insert(pool, account_id, &verified, store_email)
        .await
        .map_err(ApiError::from_repo)?;
    tracing::Span::current().record("account_id", account_id);
    tracing::Span::current().record("password_cleared", linked.password_cleared);
    note("link");
    signed_in(state, account_id, None, fence, true, now).await
}

/// Checks the password of `name` and returns the account id. The length bound
/// comes first, so no oversized password reaches the hash. The name is never
/// logged.
async fn check_password(pool: &PgPool, name: &str, password: &str) -> Result<u32, ApiError> {
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(ApiError::InvalidCredentials);
    }
    match verify_account_password(pool, name, password).await {
        PasswordCheck::Ok { account_id } => Ok(account_id),
        PasswordCheck::Invalid => Err(ApiError::InvalidCredentials),
        PasswordCheck::Disabled => Err(ApiError::AccountDisabled),
        PasswordCheck::DbError => Err(ApiError::Unavailable),
    }
}

/// Turns the body's result into the response and records the refusal on the
/// span. A success recorded its own outcome inside the body.
fn answered(
    span: &tracing::Span,
    started: Instant,
    result: Result<Response, ApiError>,
) -> Response {
    let response = match result {
        Ok(response) => response,
        Err(error) => {
            let outcome = if error == ApiError::InvalidCredentials {
                "invalid_credentials"
            } else {
                "refused"
            };
            span.record("outcome", outcome);
            span.record("reason", error.code());
            error.into_response()
        }
    };
    span.record("latency_ms", started.elapsed().as_millis() as u64);
    response
}

/// The database pool, or `unavailable` when none is configured.
fn db_pool(state: &RouteState) -> Result<&PgPool, ApiError> {
    state.deps.db.as_deref().ok_or(ApiError::Unavailable)
}

#[cfg(test)]
#[path = "link_tests_guards_live_db.rs"]
mod tests_guards_live_db;
#[cfg(test)]
#[path = "link_tests_live_db.rs"]
mod tests_live_db;

#[cfg(test)]
mod answered_tests {
    use super::*;
    use crate::telemetry::RecordedFields;
    use serde_json::json;

    /// A refused link records its reason and latency on the span.
    #[test]
    fn refusal_records_reason_and_latency() {
        let recorder = RecordedFields::default();
        recorder.scope(|| {
            let span = telemetry::idp_link();
            answered(&span, Instant::now(), Err(ApiError::InvalidRequest));
        });
        let names = recorder.names();
        assert!(names.contains(&"reason"), "a refusal lacks its reason");
        assert!(names.contains(&"latency_ms"), "a refusal lacks latency");
    }

    /// A signed-in link records its latency on the span.
    #[test]
    fn success_records_latency() {
        let recorder = RecordedFields::default();
        recorder.scope(|| {
            let span = telemetry::idp_link();
            let response = Json(json!({"result": "signed_in"})).into_response();
            answered(&span, Instant::now(), Ok(response));
        });
        assert!(recorder.names().contains(&"latency_ms"));
    }
}

//! `POST /token` and `POST /logout` (contract C6): the handoff code and the
//! refresh grants, and the sign-out that ends a refresh family.
//!
//! A sign-in ends in [`signed_in`], which CT-26 and CT-30 reuse. The wall
//! clock is read here, in the route layer (contract C12).

use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::Instrument;

use crate::limits::Bucket;
use crate::link::{self, LinkFacts, LinkHint, Refusal, Resolution};
use crate::repo::event::{self, EventKind};
use crate::repo::refresh::{self, Issued, Rotation};
use crate::repo::{account, identity};
use crate::routes::attach::attach;
use crate::routes::ctx::{client_ip, note, RouteState};
use crate::routes::error::ApiError;
use crate::stores::{HandoffOutcome, ACCESS_LIFE, LINK_GRANT_LIFE};
use crate::telemetry;

/// The `POST /token` body (C6). Every field is optional here so that a missing
/// one is our `400 invalid_request`, not the extractor's.
#[derive(Deserialize)]
pub struct TokenRequest {
    grant_type: Option<String>,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    refresh_token: Option<String>,
}

/// The `POST /logout` body (C6).
#[derive(Deserialize)]
pub struct LogoutRequest {
    refresh_token: Option<String>,
}

/// `POST /token`: redeems a handoff code or rotates a refresh token (C6).
///
/// The per-IP `Token` limit counts every request before any code is redeemed
/// or any token minted.
pub async fn token(
    State(state): State<RouteState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    body: Result<Json<TokenRequest>, JsonRejection>,
) -> Response {
    let span = telemetry::idp_token();
    let started = Instant::now();
    let fence = state.ctx.stores.access.fence();
    let result = answer(&state, peer, body, fence)
        .instrument(span.clone())
        .await;
    match result {
        Ok(response) => {
            span.record("latency_ms", started.elapsed().as_millis() as u64);
            response
        }
        Err(error) => {
            span.record("outcome", "refused");
            span.record("reason", error.code());
            span.record("latency_ms", started.elapsed().as_millis() as u64);
            error.into_response()
        }
    }
}

/// The body of [`token`]: the limit, the request shape, then the grant. `fence`
/// is the access-token fence captured when the request arrived (C16.3).
async fn answer(
    state: &RouteState,
    peer: SocketAddr,
    body: Result<Json<TokenRequest>, JsonRejection>,
    fence: u64,
) -> Result<Response, ApiError> {
    let now = Instant::now();
    state
        .ctx
        .limit(Bucket::Token, &client_ip(ConnectInfo(peer)), now)?;
    let Json(request) = body.map_err(|_| ApiError::InvalidRequest)?;
    let pool = state.deps.db.as_deref().ok_or(ApiError::Unavailable)?;
    match request.grant_type.as_deref() {
        Some("authorization_code") => code_grant(state, pool, &request, fence, now).await,
        Some("refresh_token") => refresh_grant(state, pool, &request, fence, now).await,
        _ => Err(ApiError::InvalidRequest),
    }
}

/// The `authorization_code` grant: redeems the handoff code and resolves the
/// identity to an account (C7).
async fn code_grant(
    state: &RouteState,
    pool: &PgPool,
    request: &TokenRequest,
    fence: u64,
    now: Instant,
) -> Result<Response, ApiError> {
    let (Some(code), Some(verifier), Some(redirect)) = (
        request.code.as_deref(),
        request.code_verifier.as_deref(),
        request.redirect_uri.as_deref(),
    ) else {
        return Err(ApiError::InvalidRequest);
    };
    let HandoffOutcome { identity, intent } = state
        .ctx
        .stores
        .handoff
        .redeem(code, verifier, redirect, now)
        .ok_or(ApiError::InvalidGrant)?;
    if let Some(intent) = intent {
        return attach(state, pool, identity, intent, fence, now).await;
    }

    let provider = identity.provider.as_str().to_owned();
    let trusted = state.trusts_email(&identity);
    let bound = identity::find_bound(pool, &provider, &identity.issuer, &identity.subject)
        .await
        .map_err(ApiError::from_repo)?;
    let bound_id = bound.as_ref().map(|(id, _)| *id);
    let email_candidates = match identity.email.as_deref() {
        Some(email) if trusted && identity.email_verified => {
            identity::email_candidates(pool, &email.to_lowercase())
                .await
                .map_err(ApiError::from_repo)?
        }
        _ => Vec::new(),
    };
    let facts = LinkFacts {
        bound_to: bound.map(|(_, account)| account),
        email_candidates,
        provider_trusted_email: trusted,
        email_verified: identity.email_verified,
    };

    match link::resolve(&facts) {
        Resolution::Existing(account_id) => {
            let id = bound_id.ok_or(ApiError::InvalidGrant)?;
            identity::touch(pool, id, Utc::now().timestamp_millis())
                .await
                .map_err(ApiError::from_repo)?;
            note("signed_in");
            signed_in(state, account_id, id, fence, true, now).await
        }
        Resolution::AutoLink(account_id) => {
            let linked = identity::insert(pool, account_id, &identity, trusted)
                .await
                .map_err(ApiError::from_repo)?;
            event::record(
                pool,
                account_id,
                Some(linked.identity_id),
                EventKind::AutoLink,
                Some(provider.as_str()),
            )
            .await
            .map_err(ApiError::from_repo)?;
            note("auto_link");
            signed_in(state, account_id, linked.identity_id, fence, true, now).await
        }
        Resolution::NeedsLink { hint } => {
            note("needs_link");
            let hint = match hint {
                LinkHint::None => "none",
                LinkHint::EmailMatch => "email_match",
            };
            let display_name = identity.display_name.clone();
            let grant = state.ctx.stores.grants.issue(identity, now);
            let cfg = &state.ctx.cfg;
            Ok(Json(json!({
                "result": "needs_link",
                "link_grant": grant,
                "expires_in": LINK_GRANT_LIFE.as_secs(),
                "identity": {"provider": provider, "display_name": display_name},
                "hint": hint,
                "signup_open": cfg.signup_open,
            }))
            .into_response())
        }
        Resolution::Refused(Refusal::AccountDisabled) => Err(ApiError::AccountDisabled),
    }
}

/// The `refresh_token` grant: rotates the presented token (C6).
async fn refresh_grant(
    state: &RouteState,
    pool: &PgPool,
    request: &TokenRequest,
    fence: u64,
    now: Instant,
) -> Result<Response, ApiError> {
    let presented = request
        .refresh_token
        .as_deref()
        .ok_or(ApiError::InvalidRequest)?;
    match refresh::rotate(pool, presented, Utc::now())
        .await
        .map_err(ApiError::from_repo)?
    {
        Rotation::Rotated {
            account_id,
            family_id,
            token,
        } => {
            let facts = account::facts(pool, account_id)
                .await
                .map_err(ApiError::from_repo)?
                .ok_or(ApiError::InvalidGrant)?;
            if !facts.enabled {
                refresh::revoke_account(pool, account_id, Utc::now())
                    .await
                    .map_err(ApiError::from_repo)?;
                state.ctx.stores.access.revoke_account(account_id);
                state.deps.launch_tickets.revoke_account(account_id);
                tracing::Span::current().record("account_id", account_id);
                note("refused");
                return Ok(ApiError::AccountDisabled.into_response());
            }
            note("signed_in");
            session(
                state,
                pool,
                account_id,
                (token, family_id),
                false,
                fence,
                now,
            )
            .await
        }
        Rotation::Reused {
            account_id,
            family_id,
        } => {
            // The revokes run before the audit event (C16.6): a failed insert
            // must not leave the presented family or the launch ticket live.
            state.ctx.stores.access.revoke_family(&family_id);
            state.deps.launch_tickets.revoke_account(account_id);
            if let Err(e) =
                event::record(pool, account_id, None, EventKind::RefreshReuse, None).await
            {
                tracing::warn!(
                    account_id,
                    reason = "reuse_audit_failed",
                    error = %e,
                    "refresh reuse audit event not recorded"
                );
            }
            tracing::Span::current().record("account_id", account_id);
            note("reuse");
            Ok(ApiError::InvalidGrant.into_response())
        }
        Rotation::Expired | Rotation::Unknown => Err(ApiError::InvalidGrant),
    }
}

/// Ends a sign-in for `account_id` through `identity_id`: a new refresh family,
/// issued only while the account is enabled and the identity linked (C16.2),
/// then the access token and the `signed_in` body (C6). CT-26 and CT-30 reuse
/// it. `fence` is the access-token fence the request captured on arrival.
pub(super) async fn signed_in(
    state: &RouteState,
    account_id: u32,
    identity_id: i64,
    fence: u64,
    via_code: bool,
    now: Instant,
) -> Result<Response, ApiError> {
    let pool = state.deps.db.as_deref().ok_or(ApiError::Unavailable)?;
    let pair = match refresh::issue_bound(pool, account_id, identity_id, Utc::now())
        .await
        .map_err(ApiError::from_repo)?
    {
        Issued::Pair(token, family_id) => (token, family_id),
        Issued::Disabled => return Err(ApiError::AccountDisabled),
        Issued::Unbound => return Err(ApiError::InvalidGrant),
    };
    session(state, pool, account_id, pair, via_code, fence, now).await
}

/// Mints the access token for a refresh token already issued and answers
/// `signed_in`. The account name is the display name too: the account row
/// holds no other name. A token minted after its family was revoked is refused
/// (`fence`, C16.3): the family is then revoked again, best effort.
pub(super) async fn session(
    state: &RouteState,
    pool: &PgPool,
    account_id: u32,
    (refresh_token, family_id): (String, String),
    via_code: bool,
    fence: u64,
    now: Instant,
) -> Result<Response, ApiError> {
    let (account_name,): (String,) =
        sqlx::query_as("SELECT account_name FROM account WHERE account_id = $1")
            .bind(account_id as i32)
            .fetch_optional(pool)
            .await
            .map_err(ApiError::from_repo)?
            .ok_or(ApiError::InvalidGrant)?;
    tracing::Span::current().record("account_id", account_id);
    let Some(access_token) = state
        .ctx
        .stores
        .access
        .mint_fenced(account_id, &family_id, via_code, now, fence)
    else {
        tracing::info!(
            target: "cimmeria_identity",
            reason = "fence_refused",
            "an access token was not minted: its family was revoked while the request ran"
        );
        if refresh::revoke_family(pool, &family_id, Utc::now())
            .await
            .is_err()
        {
            tracing::warn!(
                target: "cimmeria_identity",
                reason = "fence_cleanup_failed",
                "the revoked family could not be cleaned up"
            );
        }
        return Err(ApiError::InvalidGrant);
    };
    Ok(Json(json!({
        "result": "signed_in",
        "access_token": access_token,
        "expires_in": ACCESS_LIFE.as_secs(),
        "refresh_token": refresh_token,
        "account": {
            "account_id": account_id,
            "account_name": account_name,
            "display_name": account_name,
        },
    }))
    .into_response())
}

/// `POST /logout`: ends the refresh family of the presented token and its
/// access tokens. Always `204`, whatever the body held (C6).
pub async fn logout(
    State(state): State<RouteState>,
    body: Result<Json<LogoutRequest>, JsonRejection>,
) -> Response {
    let presented = body.ok().and_then(|Json(request)| request.refresh_token);
    if let (Some(presented), Some(pool)) = (presented, state.deps.db.as_deref()) {
        if end_session(&state, pool, &presented).await.is_err() {
            tracing::warn!(
                target: "cimmeria_identity",
                reason = "logout_failed",
                "logout could not revoke the session"
            );
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

/// Revokes the family of `presented`, the family's access tokens and the
/// account's launch ticket.
async fn end_session(state: &RouteState, pool: &PgPool, presented: &str) -> Result<(), ApiError> {
    let digest = Sha256::digest(presented.as_bytes());
    let family: Option<(String, i32)> = sqlx::query_as(
        "SELECT family_id, account_id FROM auth_refresh_token WHERE token_hash = $1",
    )
    .bind(&digest[..])
    .fetch_optional(pool)
    .await
    .map_err(ApiError::from_repo)?;
    refresh::revoke_token(pool, presented, Utc::now())
        .await
        .map_err(ApiError::from_repo)?;
    if let Some((family_id, account_id)) = family {
        state.ctx.stores.access.revoke_family(&family_id);
        state.deps.launch_tickets.revoke_account(account_id as u32);
    }
    Ok(())
}

#[cfg(test)]
#[path = "token_tests_live_db.rs"]
pub(crate) mod tests_live_db;

#[cfg(test)]
#[path = "token_reuse_tests_live_db.rs"]
mod token_reuse_tests_live_db;

#[cfg(test)]
#[path = "token_fence_tests_live_db.rs"]
mod token_fence_tests_live_db;

/// Span tests of the token and ticket routes (C11), on a state with no
/// database. Shared with the launch-ticket tests.
#[cfg(test)]
pub(crate) mod span_tests {
    use std::sync::Arc;

    use cimmeria_auth::auth::{AuthRouteDeps, LaunchTicketStore};

    use super::*;
    use crate::config;
    use crate::limits::RateLimiter;
    use crate::link_code::LinkCodeStore;
    use crate::providers::Providers;
    use crate::routes::ctx::IdentityCtx;
    use crate::stores::Stores;
    use crate::telemetry::RecordedFields;

    const TABLE: &str = r#"
public_base_url = "https://play.example.org"

[links]
website = "https://example.org"
account = "https://example.org/account"
forums = "https://example.org/forums"
"#;

    /// A route state with no database, no providers and the default limits.
    pub(crate) fn test_state() -> RouteState {
        let (cfg, _) = config::load(TABLE, &|_| None, false).expect("the fixture loads");
        RouteState {
            deps: Arc::new(AuthRouteDeps {
                db: None,
                launch_tickets: Arc::new(LaunchTicketStore::default()),
                developer_mode: false,
            }),
            ctx: IdentityCtx {
                limiter: Arc::new(RateLimiter::with_limits(&cfg.limits)),
                cfg: Arc::new(cfg),
                providers: Arc::new(Providers::from_parts(Vec::new(), None)),
                stores: Arc::new(Stores::default()),
                link_codes: Box::leak(Box::new(LinkCodeStore::default())),
            },
        }
    }

    /// A refused `/token` records its reason and latency on the span.
    #[tokio::test]
    async fn refusal_records_reason_and_latency() {
        let recorder = RecordedFields::default();
        let _subscriber = tracing::subscriber::set_default(recorder.clone());
        let body = TokenRequest {
            grant_type: Some("bogus".to_owned()),
            code: None,
            code_verifier: None,
            redirect_uri: None,
            refresh_token: None,
        };
        let peer = SocketAddr::from(([127, 0, 0, 1], 40_000));
        let _response = token(State(test_state()), ConnectInfo(peer), Ok(Json(body))).await;
        let names = recorder.names();
        assert!(
            names.contains(&"reason"),
            "a refused token lacks its reason"
        );
        assert!(
            names.contains(&"latency_ms"),
            "a refused token lacks latency"
        );
    }
}

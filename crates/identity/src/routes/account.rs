//! The signed-in account routes (contract C6): `link/start`, `GET /account`
//! and `DELETE /identities/{identity_id}`.
//!
//! Every route takes the [`Bearer`] grant first, so a missing or unknown token
//! is `401 invalid_token` before anything else runs. Unlinking is the one
//! action that can leave an account with no way in, so it needs a fresh code
//! sign-in and counts the methods under a row lock (contract C15 e).

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use tracing::Instrument;
use url::Url;

use crate::limits::Bucket;
use crate::repo::event::{self, EventKind};
use crate::repo::{account, identity, refresh};
use crate::routes::bearer::Bearer;
use crate::routes::ctx::{client_ip, RouteState};
use crate::routes::error::ApiError;
use crate::routes::signin::{record_outcome, valid_challenge, valid_redirect_uri, valid_state};
use crate::stores::{AccessGrant, LinkIntent};
use crate::telemetry;

/// How recent the token's code sign-in must be to unlink (C6). A refresh-minted
/// token does not qualify, so a stolen refresh token cannot remove a method.
const REAUTH_LIFE: Duration = Duration::from_secs(10 * 60);

/// Whether `grant` is a code sign-in token no older than `REAUTH_LIFE` at `now`.
/// Adding a method needs it as much as removing one: a method added from a
/// stolen token would otherwise pass the unlink check (W3-1).
fn fresh_code_token(grant: &AccessGrant, now: Instant) -> bool {
    grant.via_code && now.duration_since(grant.issued_at) <= REAUTH_LIFE
}

/// The body of `POST /link/start` (C6). Every field is optional here so that a
/// missing one is our `400 invalid_request`.
#[derive(Deserialize)]
pub struct LinkStartRequest {
    provider: Option<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

/// `POST /link/start`: a signed-in player asks to add a provider. The answer is
/// an `/authorize` URL carrying a single-use `intent` that names the account
/// (C6). The body is validated like `authorize`'s query. Needs a fresh code
/// sign-in, like unlink (W3-1).
pub async fn link_start(
    State(state): State<RouteState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Bearer(grant): Bearer,
    body: Result<Json<LinkStartRequest>, JsonRejection>,
) -> Response {
    let span = telemetry::idp_link_start();
    let started = Instant::now();
    let result = start_link(&state, peer, &grant, body)
        .instrument(span.clone())
        .await;
    let response = result.unwrap_or_else(IntoResponse::into_response);
    record_outcome(&span, started, &response);
    response
}

/// The work of `link_start`.
async fn start_link(
    state: &RouteState,
    peer: SocketAddr,
    grant: &AccessGrant,
    body: Result<Json<LinkStartRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let now = Instant::now();
    if !fresh_code_token(grant, now) {
        return Err(ApiError::ReauthRequired);
    }
    let Json(request) = body.map_err(|_| ApiError::InvalidRequest)?;
    let provider = request
        .provider
        .as_deref()
        .ok_or(ApiError::InvalidRequest)?;
    let redirect_uri = request
        .redirect_uri
        .as_deref()
        .filter(|uri| valid_redirect_uri(uri))
        .ok_or(ApiError::InvalidRequest)?;
    let launcher_state = request
        .state
        .as_deref()
        .filter(|s| valid_state(s))
        .ok_or(ApiError::InvalidRequest)?;
    let challenge = request
        .code_challenge
        .as_deref()
        .filter(|c| valid_challenge(c))
        .ok_or(ApiError::InvalidRequest)?;
    if request.code_challenge_method.as_deref() != Some("S256") {
        return Err(ApiError::InvalidRequest);
    }
    let ip = client_ip(ConnectInfo(peer));
    state.ctx.limit(Bucket::Authorize, &ip, now)?;
    state
        .ctx
        .providers
        .get(provider)
        .ok_or(ApiError::UnknownProvider)?;
    let pool = state.deps.db.as_deref().ok_or(ApiError::Unavailable)?;
    let facts = account::facts(pool, grant.account_id)
        .await
        .map_err(ApiError::from_repo)?
        .ok_or(ApiError::InvalidToken)?;
    if !facts.enabled {
        return Err(ApiError::AccountDisabled);
    }
    let intent = state.ctx.stores.intents.issue(
        LinkIntent {
            account_id: grant.account_id,
        },
        now,
    );
    let mut url = Url::parse(&format!(
        "{}/auth/v1/authorize",
        state.ctx.cfg.public_base_url
    ))
    .map_err(|_| ApiError::Unavailable)?;
    url.query_pairs_mut()
        .append_pair("provider", provider)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", launcher_state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("intent", &intent);
    Ok(Json(json!({"authorize_url": url.as_str()})).into_response())
}

/// Refuses a disabled account with `403 account_disabled`, the answer `mint`
/// and `link_start` give (C6). A row that is gone is `invalid_token`.
pub(super) async fn require_enabled(pool: &sqlx::PgPool, account_id: u32) -> Result<(), ApiError> {
    let facts = account::facts(pool, account_id)
        .await
        .map_err(ApiError::from_repo)?
        .ok_or(ApiError::InvalidToken)?;
    if facts.enabled {
        Ok(())
    } else {
        Err(ApiError::AccountDisabled)
    }
}

/// `GET /account`: the account's name, its display name and its linked
/// identities. The display name is the most recently used identity's, else the
/// account name. Never a subject or an email (C6, C15).
pub async fn get_account(
    State(state): State<RouteState>,
    Bearer(grant): Bearer,
) -> Result<Response, ApiError> {
    let pool = state.deps.db.as_deref().ok_or(ApiError::Unavailable)?;
    let account_id = grant.account_id;
    require_enabled(pool, account_id).await?;
    let name = account_name(pool, account_id)
        .await
        .map_err(ApiError::from_repo)?
        .ok_or(ApiError::InvalidToken)?;
    let rows = identity::list_for_account(pool, account_id)
        .await
        .map_err(ApiError::from_repo)?;
    let display_name = rows
        .iter()
        .max_by_key(|row| row.last_used_at)
        .map_or_else(|| name.clone(), |row| row.display_name.clone());
    let identities: Vec<_> = rows
        .iter()
        .map(|row| {
            json!({
                "identity_id": row.identity_id,
                "provider": row.provider,
                "display_name": row.display_name,
                "created_at": row.created_at,
                "last_used_at": row.last_used_at,
            })
        })
        .collect();
    Ok(Json(json!({
        "account_id": account_id,
        "account_name": name,
        "display_name": display_name,
        "identities": identities,
    }))
    .into_response())
}

/// `DELETE /identities/{identity_id}`: removes one linked sign-in method (C6).
/// Needs a token from a code sign-in less than 10 minutes old.
pub async fn unlink(
    State(state): State<RouteState>,
    Bearer(grant): Bearer,
    identity_id: Result<Path<i64>, PathRejection>,
) -> Response {
    let span = telemetry::idp_unlink(grant.account_id);
    let started = Instant::now();
    let result = match identity_id {
        // A path that is not a number names no row: the same answer as a miss.
        Err(_) => Err(ApiError::NotFound),
        Ok(Path(identity_id)) => remove_identity(&state, &grant, identity_id)
            .instrument(span.clone())
            .await
            .map(|()| StatusCode::NO_CONTENT.into_response()),
    };
    let response = result.unwrap_or_else(IntoResponse::into_response);
    record_outcome(&span, started, &response);
    response
}

/// The work of `unlink`. The count and the delete share one transaction that
/// holds the account row lock, so two unlinks of one account run one after the
/// other and the second sees the first. Without the lock both could read two
/// methods and both delete, leaving none (C15 e).
///
/// After the delete it ends every other session of the account: the refresh
/// families except the caller's, and the access tokens and launch tickets
/// except the caller's own family (client-tls-oidc D1).
async fn remove_identity(
    state: &RouteState,
    grant: &AccessGrant,
    identity_id: i64,
) -> Result<(), ApiError> {
    let now = Instant::now();
    if !fresh_code_token(grant, now) {
        return Err(ApiError::ReauthRequired);
    }
    let pool = state.deps.db.as_deref().ok_or(ApiError::Unavailable)?;
    let account_id = grant.account_id;
    require_enabled(pool, account_id).await?;
    let mut tx = pool.begin().await.map_err(ApiError::from_repo)?;
    let locked: Option<(i32,)> =
        sqlx::query_as("SELECT account_id FROM account WHERE account_id = $1 FOR UPDATE")
            .bind(account_id as i32)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::from_repo)?;
    if locked.is_none() {
        return Err(ApiError::InvalidToken);
    }
    let (methods,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM account_identity WHERE account_id = $1")
            .bind(account_id as i32)
            .fetch_one(&mut *tx)
            .await
            .map_err(ApiError::from_repo)?;
    if methods <= 1 {
        return Err(ApiError::LastMethod);
    }
    let removed: Option<(String,)> = sqlx::query_as(
        "DELETE FROM account_identity WHERE account_id = $1 AND identity_id = $2 \
         RETURNING provider",
    )
    .bind(account_id as i32)
    .bind(identity_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::from_repo)?;
    let (provider,) = removed.ok_or(ApiError::NotFound)?;
    event::record_tx(
        &mut tx,
        account_id,
        Some(identity_id),
        EventKind::Unlink,
        Some(&provider),
    )
    .await
    .map_err(ApiError::from_repo)?;
    tx.commit().await.map_err(ApiError::from_repo)?;
    state
        .ctx
        .stores
        .access
        .revoke_account_except(account_id, &grant.family_id);
    state.deps.launch_tickets.revoke_account(account_id);
    refresh::revoke_account_except(pool, account_id, &grant.family_id, Utc::now())
        .await
        .map_err(ApiError::from_repo)?;
    tracing::Span::current().record("provider", provider.as_str());
    Ok(())
}

/// The account's name, or `None` when no row has `account_id`. Shared with the
/// launch-ticket mint, which needs the name the ticket is bound to.
pub(super) async fn account_name(
    pool: &PgPool,
    account_id: u32,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT account_name FROM account WHERE account_id = $1")
            .bind(account_id as i32)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(name,)| name))
}

#[cfg(test)]
#[path = "account_tests_live_db.rs"]
mod tests_live_db;

#[cfg(test)]
#[path = "account_disabled_tests_live_db.rs"]
mod disabled_tests_live_db;

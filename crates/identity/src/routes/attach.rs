//! The `/link/start` attach (C6, C16.5): the callback of a link intent adds a
//! provider identity to the account that started it, and only while that
//! account may still be linked and the session that started it is still live.

use std::time::Instant;

use axum::response::Response;
use chrono::Utc;
use sqlx::PgPool;

use crate::link::{self, AccountFacts};
use crate::provider::VerifiedIdentity;
use crate::repo::{identity, refresh};
use crate::routes::ctx::{note, RouteState};
use crate::routes::error::ApiError;
use crate::routes::token::session;
use crate::stores::LinkIntent;

/// A `/link/start` intent: the verified identity attaches to the intent's
/// account, in one transaction that holds the account row `FOR UPDATE`.
///
/// The account must exist (`invalid_grant`), be enabled (`account_disabled`) and
/// be linkable, so a staff or test account refuses with `not_linkable` even when
/// it became one after `/link/start` (C12). The intent's family must still be
/// live (`invalid_grant`), so an unlink or a revoke after `/link/start` stops
/// the attach (C16.5). An identity bound to another account is refused
/// (`invalid_grant`); bound to this one it just signs in. A new family is issued
/// in the same transaction, then the sign-in answers.
pub(super) async fn attach(
    state: &RouteState,
    pool: &PgPool,
    verified: VerifiedIdentity,
    intent: LinkIntent,
    fence: u64,
    now: Instant,
) -> Result<Response, ApiError> {
    let LinkIntent {
        account_id,
        family_id,
    } = intent;
    let mut tx = pool.begin().await.map_err(ApiError::from_repo)?;

    let row: Option<(bool, i32, bool)> = sqlx::query_as(
        "SELECT enabled, accesslevel, test_account FROM account \
         WHERE account_id = $1 FOR UPDATE",
    )
    .bind(account_id as i32)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::from_repo)?;
    let facts = match row {
        Some((enabled, accesslevel, test_account)) => AccountFacts {
            account_id,
            enabled,
            accesslevel,
            test_account,
        },
        None => return Err(ApiError::InvalidGrant),
    };
    if !facts.enabled {
        return Err(ApiError::AccountDisabled);
    }
    if !link::linkable(&facts) {
        return Err(ApiError::NotLinkable);
    }

    let (live,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM auth_refresh_token \
         WHERE family_id = $1 AND account_id = $2 AND revoked_at IS NULL \
         AND family_expires_at > now())",
    )
    .bind(&family_id)
    .bind(account_id as i32)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::from_repo)?;
    if !live {
        tracing::Span::current().record("account_id", account_id);
        tracing::info!(
            target: "cimmeria_identity",
            reason = "intent_family_not_live",
            "a link intent was refused: the session that started it is no longer live"
        );
        return Err(ApiError::InvalidGrant);
    }

    let provider = verified.provider.as_str();
    let found = identity::find_bound_on(&mut tx, provider, &verified.issuer, &verified.subject)
        .await
        .map_err(ApiError::from_repo)?;
    match found {
        Some((_, bound)) if bound.account_id == account_id => {}
        Some(_) => return Err(ApiError::InvalidGrant),
        None => {
            let trusted = state.trusts_email(&verified) && verified.email_verified;
            identity::insert_on(&mut tx, account_id, &verified, trusted)
                .await
                .map_err(ApiError::from_repo)?;
        }
    }
    let pair = refresh::issue_on(&mut tx, account_id, None, Utc::now())
        .await
        .map_err(ApiError::from_repo)?;
    tx.commit().await.map_err(ApiError::from_repo)?;

    note("signed_in");
    session(state, pool, account_id, pair, true, fence, now).await
}

#[cfg(test)]
#[path = "attach_tests_live_db.rs"]
mod attach_tests_live_db;

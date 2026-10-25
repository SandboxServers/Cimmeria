//! Linked identities: the `account_identity` rows (contract C5 and C15).
//!
//! [`insert`] is the one write that links a provider identity to an account.
//! It clears the account's password in the same transaction (C15 a), so an
//! account with an identity never holds one.

use sqlx::PgPool;

use super::account::{facts_of, FactsRow};
use super::event::{self, EventKind};
use super::RepoError;
use crate::link::AccountFacts;
use crate::provider::VerifiedIdentity;

/// The columns [`list_for_account`] selects. Holds neither the subject nor the
/// email, so the account page can never show them (R7, C6).
pub(super) const LIST_SQL: &str = "SELECT identity_id, provider, display_name, \
     (EXTRACT(EPOCH FROM created_at) * 1000)::BIGINT, \
     (EXTRACT(EPOCH FROM last_used_at) * 1000)::BIGINT \
     FROM account_identity WHERE account_id = $1 ORDER BY identity_id";

/// One linked identity as the account page shows it (C6 `GET /account`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityRow {
    /// The row's id, which unlink and the client name it by.
    pub identity_id: i64,
    /// The provider's id, as config names it.
    pub provider: String,
    /// The display name the provider gave, sanitized at sign-in.
    pub display_name: String,
    /// When the identity was linked, in Unix milliseconds.
    pub created_at: i64,
    /// When the identity last signed in, in Unix milliseconds.
    pub last_used_at: i64,
}

/// What [`insert`] did: the new identity's id, and whether a password was
/// cleared on the way (C15 a; the caller tells the player when it was).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Linked {
    /// The new `identity_id`.
    pub identity_id: i64,
    /// Whether the account had a password, which this link cleared.
    pub password_cleared: bool,
}

/// Finds the account bound to a provider identity, with the account's facts.
/// Returns the identity's id and the account's facts.
pub async fn find_bound(
    pool: &PgPool,
    provider: &str,
    issuer: &str,
    subject: &str,
) -> Result<Option<(i64, AccountFacts)>, RepoError> {
    let row: Option<(i64, i32, bool, i32, bool)> = sqlx::query_as(
        "SELECT i.identity_id, a.account_id, a.enabled, a.accesslevel, a.test_account \
         FROM account_identity i JOIN account a ON a.account_id = i.account_id \
         WHERE i.provider = $1 AND i.issuer = $2 AND i.subject = $3",
    )
    .bind(provider)
    .bind(issuer)
    .bind(subject)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(identity_id, id, enabled, level, test)| {
        (identity_id, facts_of(id, enabled, level, test))
    }))
}

/// The distinct accounts with a verified identity whose email is `email_lower`.
/// An identity whose email was never verified is not a candidate, so the
/// sign-in can never auto-link on an unproven address.
pub async fn email_candidates(
    pool: &PgPool,
    email_lower: &str,
) -> Result<Vec<AccountFacts>, RepoError> {
    let rows: Vec<FactsRow> = sqlx::query_as(
        "SELECT DISTINCT a.account_id, a.enabled, a.accesslevel, a.test_account \
         FROM account_identity i JOIN account a ON a.account_id = i.account_id \
         WHERE lower(i.email) = $1 AND i.email_verified_at IS NOT NULL \
         ORDER BY a.account_id",
    )
    .bind(email_lower)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, enabled, level, test)| facts_of(id, enabled, level, test))
        .collect())
}

/// Links a verified provider identity to `account_id` in one transaction.
///
/// The account row is locked first. When it holds a password, both password
/// columns are cleared, `password_algo` is reset to 1 (the schema default, which
/// no longer matches any stored hash), and a `password_cleared` event is
/// written (C15 a). Then the identity is inserted and a `link` event follows.
///
/// The email is stored only when `store_email` is set. Its verification time
/// is set only when the provider verified it too, so [`email_candidates`]
/// sees it only then.
///
/// A repeated `(provider, issuer, subject)` returns [`RepoError::Conflict`],
/// and so does a C5 guard refusal; either way the transaction rolls back and
/// the password stays.
pub async fn insert(
    pool: &PgPool,
    account_id: u32,
    identity: &VerifiedIdentity,
    store_email: bool,
) -> Result<Linked, RepoError> {
    let account = account_id as i32;
    let provider = identity.provider.as_str();
    let mut tx = pool.begin().await?;

    let locked: Option<(bool,)> = sqlx::query_as(
        "SELECT (password IS NOT NULL OR password_hash_v2 IS NOT NULL) \
         FROM account WHERE account_id = $1 FOR UPDATE",
    )
    .bind(account)
    .fetch_optional(&mut *tx)
    .await?;
    // A missing account has no password; the identity insert then fails its
    // foreign key, which maps to `Db`.
    let password_cleared = locked.is_some_and(|(has,)| has);
    if password_cleared {
        sqlx::query(
            "UPDATE account SET password = NULL, password_hash_v2 = NULL, password_algo = 1 \
             WHERE account_id = $1",
        )
        .bind(account)
        .execute(&mut *tx)
        .await?;
        event::record_tx(
            &mut tx,
            account_id,
            None,
            EventKind::PasswordCleared,
            Some(provider),
        )
        .await?;
    }

    let stored_email = if store_email {
        identity.email.as_deref()
    } else {
        None
    };
    let verified_now = stored_email.is_some() && identity.email_verified;
    let display_name = VerifiedIdentity::sanitized_display_name(&identity.display_name);
    let (identity_id,): (i64,) = sqlx::query_as(
        "INSERT INTO account_identity \
         (account_id, provider, issuer, subject, display_name, email, email_verified_at) \
         VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $7 THEN now() END) RETURNING identity_id",
    )
    .bind(account)
    .bind(provider)
    .bind(&identity.issuer)
    .bind(&identity.subject)
    .bind(display_name)
    .bind(stored_email)
    .bind(verified_now)
    .fetch_one(&mut *tx)
    .await?;
    event::record_tx(
        &mut tx,
        account_id,
        Some(identity_id),
        EventKind::Link,
        Some(provider),
    )
    .await?;

    tx.commit().await?;
    Ok(Linked {
        identity_id,
        password_cleared,
    })
}

/// Records that an identity signed in now, by its id.
pub async fn touch(pool: &PgPool, identity_id: i64, now_ms: i64) -> Result<(), RepoError> {
    sqlx::query(
        "UPDATE account_identity SET last_used_at = TO_TIMESTAMP($2::BIGINT / 1000.0) \
         WHERE identity_id = $1",
    )
    .bind(identity_id)
    .bind(now_ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// The account's linked identities, oldest first. Never the subject or email.
pub async fn list_for_account(
    pool: &PgPool,
    account_id: u32,
) -> Result<Vec<IdentityRow>, RepoError> {
    let rows: Vec<(i64, String, String, i64, i64)> = sqlx::query_as(LIST_SQL)
        .bind(account_id as i32)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(identity_id, provider, display_name, created_at, last_used_at)| IdentityRow {
                identity_id,
                provider,
                display_name,
                created_at,
                last_used_at,
            },
        )
        .collect())
}

/// Removes one identity from `account_id`. Returns whether a row was removed.
/// The caller checks [`count_methods`] first (C15 e).
pub async fn delete(pool: &PgPool, account_id: u32, identity_id: i64) -> Result<bool, RepoError> {
    let done =
        sqlx::query("DELETE FROM account_identity WHERE account_id = $1 AND identity_id = $2")
            .bind(account_id as i32)
            .bind(identity_id)
            .execute(pool)
            .await?;
    Ok(done.rows_affected() > 0)
}

/// How many sign-in methods the account has. Identities only: a linked
/// account holds no password (C15 e), so a password never counts.
pub async fn count_methods(pool: &PgPool, account_id: u32) -> Result<i64, RepoError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM account_identity WHERE account_id = $1")
            .bind(account_id as i32)
            .fetch_one(pool)
            .await?;
    Ok(count)
}

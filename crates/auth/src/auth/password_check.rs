//! Password check for the account-link flow (CT-26 `link/existing`).
//!
//! This is a pure check: it never rewrites the stored password. A legacy
//! (algo 1) row keeps its SHA-1 hash, so an account whose link is refused still
//! logs in with the unmodified client, which only sends `SHA1HEX` (D-CT7).
//!
//! The answer carries no oracle. Every path spends exactly one argon2id verify,
//! whether the name is unknown, the row is legacy, or the password is wrong, so
//! the timing does not show which case it is. The decision is sync and free of
//! DB access, so CT-99 can run it off the async workers.

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use sqlx::PgPool;
use subtle::ConstantTimeEq;

use super::credentials::{sha1_upper_hex, ALGO_ARGON2ID, ALGO_SHA1_LEGACY};
use super::password_hash::{hash_argon2id, verify_argon2id};

/// Counts the argon2id verifies this module runs, so tests can assert the cost
/// of a path. nextest runs each test in its own process, so the count is not
/// shared between tests.
#[cfg(test)]
pub(super) static ARGON2_VERIFIES: AtomicUsize = AtomicUsize::new(0);

/// The argon2id hash an unknown account name is checked against. It is built
/// once, with the same cost as a real row, so a miss takes as long as a wrong
/// password on an argon2id account.
fn dummy_argon2id_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_argon2id("cimmeria-unknown-account").unwrap_or_default())
}

/// The one argon2id verify that every path of [`check_row`] spends.
fn argon2_verify(plaintext: &str, phc: &str) -> bool {
    #[cfg(test)]
    ARGON2_VERIFIES.fetch_add(1, Ordering::SeqCst);
    verify_argon2id(plaintext, phc)
}

/// Outcome of [`verify_account_password`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordCheck {
    /// The password is right for the account.
    Ok { account_id: u32 },
    /// The password is wrong, or there is no such account.
    Invalid,
    /// The account is disabled.
    Disabled,
    /// The database query failed.
    DbError,
}

/// The `account` columns the password check reads.
#[derive(sqlx::FromRow)]
struct AccountRow {
    account_id: i32,
    password: Option<String>,
    password_hash_v2: Option<String>,
    password_algo: i16,
    enabled: bool,
}

/// Checks `plaintext` against `account_name` without migrating the stored
/// hash. A legacy row is compared with the uppercase SHA-1 hex of `plaintext`
/// (the form the client stores); an argon2id row with `plaintext` itself.
pub async fn verify_account_password(
    db: &PgPool,
    account_name: &str,
    plaintext: &str,
) -> PasswordCheck {
    let row: Option<AccountRow> = match sqlx::query_as(
        "SELECT account_id, password, password_hash_v2, password_algo, enabled \
         FROM account WHERE account_name = $1",
    )
    .bind(account_name)
    .fetch_optional(db)
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!(error = %e, "password check: DB query failed");
            return PasswordCheck::DbError;
        }
    };
    check_row(row.as_ref(), plaintext)
}

/// Decides the answer for one row, or for no row, without DB access.
///
/// The password is judged first and the disabled flag second, so a disabled
/// account answers `Disabled` only to the right password. Every path spends one
/// [`argon2_verify`]: a row with no usable argon2id hash, a legacy row and an
/// unknown scheme verify against the dummy hash and are never right by it.
fn check_row(row: Option<&AccountRow>, plaintext: &str) -> PasswordCheck {
    let Some(row) = row else {
        let _ = argon2_verify(plaintext, dummy_argon2id_hash());
        return PasswordCheck::Invalid;
    };
    let right = match row.password_algo {
        ALGO_ARGON2ID => match row.password_hash_v2.as_deref() {
            Some(phc) => argon2_verify(plaintext, phc),
            None => {
                tracing::error!(
                    account_id = row.account_id,
                    reason = "argon2_hash_missing",
                    "account marked argon2id but password_hash_v2 is NULL"
                );
                let _ = argon2_verify(plaintext, dummy_argon2id_hash());
                false
            }
        },
        ALGO_SHA1_LEGACY => {
            let _ = argon2_verify(plaintext, dummy_argon2id_hash());
            let stored = row.password.as_deref().unwrap_or_default();
            !stored.is_empty()
                && bool::from(
                    stored
                        .to_ascii_uppercase()
                        .as_bytes()
                        .ct_eq(sha1_upper_hex(plaintext).as_bytes()),
                )
        }
        algo => {
            tracing::error!(
                account_id = row.account_id,
                password_algo = algo,
                reason = "unknown_password_algo",
                "account has an unrecognised password_algo"
            );
            let _ = argon2_verify(plaintext, dummy_argon2id_hash());
            false
        }
    };
    if !right {
        PasswordCheck::Invalid
    } else if !row.enabled {
        PasswordCheck::Disabled
    } else {
        PasswordCheck::Ok {
            account_id: row.account_id as u32,
        }
    }
}

#[cfg(test)]
#[path = "password_check_tests_live_db.rs"]
mod tests_live_db;

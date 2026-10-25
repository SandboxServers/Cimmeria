//! Password check for the account-link flow (CT-26 `link/existing`).
//!
//! This is a pure check: it never rewrites the stored password. A legacy
//! (algo 1) row keeps its SHA-1 hash, so an account whose link is refused still
//! logs in with the unmodified client, which only sends `SHA1HEX` (D-CT7).

use std::sync::OnceLock;

use sqlx::PgPool;

use super::credentials::{
    sha1_upper_hex, validate_credentials, AuthCredError, ClientCredential, ALGO_ARGON2ID,
    ALGO_SHA1_LEGACY,
};
use super::password_hash::{hash_argon2id, verify_argon2id};

/// The argon2id hash an unknown account name is checked against. It is built
/// once, with the same cost as a real row, so a miss takes as long as a wrong
/// password on an argon2id account.
fn dummy_argon2id_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_argon2id("cimmeria-unknown-account").unwrap_or_default())
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

/// Checks `plaintext` against `account_name` without migrating the stored
/// hash. A legacy row is compared with the uppercase SHA-1 hex of `plaintext`
/// (the form the client stores); an argon2id row with `plaintext` itself.
pub async fn verify_account_password(
    db: &PgPool,
    account_name: &str,
    plaintext: &str,
) -> PasswordCheck {
    let algo: Option<i16> =
        match sqlx::query_scalar("SELECT password_algo FROM account WHERE account_name = $1")
            .bind(account_name)
            .fetch_optional(db)
            .await
        {
            Ok(algo) => algo,
            Err(e) => {
                tracing::error!(error = %e, "password check: DB query failed");
                return PasswordCheck::DbError;
            }
        };
    let Some(algo) = algo else {
        // Spend the same argon2id verify a known argon2id account would, so the
        // answer for an unknown name is not visibly faster than a wrong password.
        let _ = verify_argon2id(plaintext, dummy_argon2id_hash());
        return PasswordCheck::Invalid;
    };

    let legacy_hash = sha1_upper_hex(plaintext);
    let credential = match algo {
        ALGO_SHA1_LEGACY => ClientCredential::LegacySha1Hex(&legacy_hash),
        ALGO_ARGON2ID => ClientCredential::Plaintext(plaintext),
        // Unknown scheme: `validate_credentials` refuses it for every credential.
        _ => ClientCredential::Plaintext(plaintext),
    };
    match validate_credentials(db, account_name, credential).await {
        Ok(acct) => PasswordCheck::Ok {
            account_id: acct.account_id,
        },
        Err(AuthCredError::InvalidCredentials) => PasswordCheck::Invalid,
        Err(AuthCredError::AccountDisabled) => PasswordCheck::Disabled,
        Err(AuthCredError::DbError(_)) => PasswordCheck::DbError,
    }
}

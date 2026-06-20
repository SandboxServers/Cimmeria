//! Credential classification, verification, and on-login password migration.
//!
//! Two credential shapes reach the login handler:
//!
//! - A **40-char uppercase-hex SHA-1 hash**, sent by the original (unpatched)
//!   client. Accepted over either HTTP or TLS.
//! - A **plaintext password**, sent by the patched client over TLS only. The
//!   handler must reject plaintext over plain HTTP — the entire argon2id
//!   rationale depends on the transport being TLS-wrapped.
//!
//! Stored passwords use one of two schemes, selected per-row by
//! `account.password_algo`:
//!
//! - `1` (`sha1_legacy`): the legacy unsalted SHA-1 hash lives in
//!   `account.password`.
//! - `2` (`argon2id`): an argon2id PHC string lives in
//!   `account.password_hash_v2`; `account.password` is NULL.
//!
//! When a legacy account authenticates with a *plaintext* password, it is
//! verified against the SHA-1 hash and then **opportunistically migrated** to
//! argon2id in the same login — no flag day, no mass re-hash.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use sha1::{Digest, Sha1};
use sqlx::PgPool;

/// Maximum accepted plaintext password length (bytes). Bounds the work argon2id
/// must do per login attempt and rejects pathological inputs.
const MAX_PLAINTEXT_LEN: usize = 128;

/// Stored-password scheme selectors (mirror `account.password_algo`).
const ALGO_SHA1_LEGACY: i16 = 1;
const ALGO_ARGON2ID: i16 = 2;

/// A credential supplied by the client, classified by shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClientCredential<'a> {
    /// 40-char uppercase-hex SHA-1 hash from the original client.
    LegacySha1Hex(&'a str),
    /// Plaintext password from the patched client (TLS only).
    Plaintext(&'a str),
}

/// Why classification of a raw credential string was rejected outright (before
/// any DB lookup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CredentialGateError {
    /// A plaintext credential arrived over plain HTTP. Plaintext is only
    /// honoured over TLS.
    PlaintextRequiresTls,
    /// A plaintext credential was empty or exceeded [`MAX_PLAINTEXT_LEN`].
    PlaintextLength,
}

/// Classify a raw credential string into a [`ClientCredential`].
///
/// - A 40-char all-hex string is a legacy SHA-1 hash — always allowed.
/// - Anything else is plaintext, which is only allowed `over_tls`, and must be
///   non-empty and at most [`MAX_PLAINTEXT_LEN`] bytes.
///
/// Factored out of the handler so the transport gate is unit-testable without
/// standing up an axum stack.
pub(super) fn classify_credential(
    raw: &str,
    over_tls: bool,
) -> Result<ClientCredential<'_>, CredentialGateError> {
    let is_legacy_hash = raw.len() == 40 && raw.chars().all(|c| c.is_ascii_hexdigit());
    if is_legacy_hash {
        return Ok(ClientCredential::LegacySha1Hex(raw));
    }

    // Everything else is treated as plaintext.
    if !over_tls {
        return Err(CredentialGateError::PlaintextRequiresTls);
    }
    if raw.is_empty() || raw.len() > MAX_PLAINTEXT_LEN {
        return Err(CredentialGateError::PlaintextLength);
    }
    Ok(ClientCredential::Plaintext(raw))
}

/// Failure modes of [`validate_credentials`].
pub(super) enum AuthCredError {
    /// Unknown account, or stored/supplied credential did not match. The same
    /// error is returned for "no such account" and "bad password" so the
    /// response never reveals which accounts exist.
    InvalidCredentials,
    /// Account exists but is disabled.
    AccountDisabled,
    /// The DB query itself failed.
    DbError(String),
}

/// Validated account info returned by [`validate_credentials`] on success.
pub(super) struct ValidatedAccount {
    pub account_id: u32,
    pub access_level: u32,
}

/// Verify `credential` against the stored password for `account_name`.
///
/// Branches on `(password_algo, credential)`:
///
/// - argon2id + plaintext → argon2id verify.
/// - argon2id + legacy hash → rejected (can't verify a stored argon2id hash
///   from a client-side SHA-1 hash).
/// - legacy + legacy hash → uppercase-hex compare (original behaviour).
/// - legacy + plaintext → recompute SHA-1, compare, and on match
///   **opportunistically migrate** the row to argon2id.
///
/// Returns the same [`AuthCredError::InvalidCredentials`] for a missing account
/// and a bad password so existence is never revealed.
pub(super) async fn validate_credentials(
    db: &PgPool,
    account_name: &str,
    credential: ClientCredential<'_>,
) -> Result<ValidatedAccount, AuthCredError> {
    #[derive(sqlx::FromRow)]
    struct AccountRow {
        account_id: i32,
        password: Option<String>,
        password_hash_v2: Option<String>,
        password_algo: i16,
        accesslevel: i32,
        enabled: bool,
    }

    let row: Option<AccountRow> = sqlx::query_as(
        "SELECT account_id, password, password_hash_v2, password_algo, accesslevel, enabled \
         FROM account WHERE account_name = $1",
    )
    .bind(account_name)
    .fetch_optional(db)
    .await
    .map_err(|e| AuthCredError::DbError(e.to_string()))?;

    let row = match row {
        Some(r) => r,
        // Don't reveal whether the account exists — same error as bad password
        // (matches C++ `BadUserPassword` behaviour).
        None => return Err(AuthCredError::InvalidCredentials),
    };

    if !row.enabled {
        return Err(AuthCredError::AccountDisabled);
    }

    let validated = ValidatedAccount {
        account_id: row.account_id as u32,
        access_level: row.accesslevel.max(0) as u32,
    };

    match (row.password_algo, credential) {
        // ── argon2id account ────────────────────────────────────────────
        (ALGO_ARGON2ID, ClientCredential::Plaintext(pw)) => {
            let stored = match row.password_hash_v2.as_deref() {
                Some(h) => h,
                None => {
                    // algo says argon2id but the column is empty — treat as a
                    // failed login rather than panic.
                    tracing::error!(
                        account_id = row.account_id,
                        reason = "argon2_hash_missing",
                        "account marked argon2id but password_hash_v2 is NULL"
                    );
                    return Err(AuthCredError::InvalidCredentials);
                }
            };
            if verify_argon2id(pw, stored) {
                Ok(validated)
            } else {
                Err(AuthCredError::InvalidCredentials)
            }
        }
        // A stored argon2id hash can't be verified against a client SHA-1 hash.
        (ALGO_ARGON2ID, ClientCredential::LegacySha1Hex(_)) => {
            Err(AuthCredError::InvalidCredentials)
        }

        // ── legacy SHA-1 account ────────────────────────────────────────
        (ALGO_SHA1_LEGACY, ClientCredential::LegacySha1Hex(client_hash)) => {
            let stored = row.password.as_deref().unwrap_or_default();
            // Compare as uppercase hex (matches C++ `upper(password)`).
            if stored.to_uppercase() == client_hash.to_uppercase() {
                Ok(validated)
            } else {
                Err(AuthCredError::InvalidCredentials)
            }
        }
        (ALGO_SHA1_LEGACY, ClientCredential::Plaintext(pw)) => {
            let stored = row.password.as_deref().unwrap_or_default();
            // Recompute the client-side SHA-1 (uppercase hex) and compare.
            if sha1_upper_hex(pw) != stored.to_uppercase() {
                return Err(AuthCredError::InvalidCredentials);
            }
            // Credential verified — opportunistically migrate to argon2id.
            // Migration failure is logged but must NOT fail the login: the
            // credential already verified, and the legacy hash still works on
            // the next attempt.
            migrate_to_argon2id(db, row.account_id, pw).await;
            Ok(validated)
        }

        // ── anything else ───────────────────────────────────────────────
        (algo, _) => {
            tracing::error!(
                account_id = row.account_id,
                password_algo = algo,
                reason = "unknown_password_algo",
                "account has an unrecognised password_algo"
            );
            Err(AuthCredError::InvalidCredentials)
        }
    }
}

/// Compute the uppercase-hex SHA-1 of `input` — the exact form the original
/// client put on the wire.
fn sha1_upper_hex(input: &str) -> String {
    let digest = Sha1::digest(input.as_bytes());
    let mut out = String::with_capacity(40);
    for byte in digest {
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

/// argon2id parameters: explicit OWASP recommendation (64 MiB, 3 iterations,
/// 1 lane). Built fresh per call; the cost is dominated by the hash itself.
fn argon2id() -> Argon2<'static> {
    // 65536 KiB = 64 MiB memory, 3 iterations, 1 degree of parallelism,
    // default output length. `Params::new` only errors on out-of-range inputs;
    // these constants are in range, so `expect` documents the invariant.
    let params =
        Params::new(65536, 3, 1, None).expect("argon2id params are within valid OWASP ranges");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// Hash a plaintext password into an argon2id PHC string with a random salt.
///
/// Returns `None` on the (practically impossible) hashing failure so callers
/// can degrade gracefully rather than panic on a login path.
fn hash_argon2id(plaintext: &str) -> Option<String> {
    let salt = SaltString::generate(&mut OsRng);
    match argon2id().hash_password(plaintext.as_bytes(), &salt) {
        Ok(hash) => Some(hash.to_string()),
        Err(e) => {
            tracing::error!(reason = "argon2_hash_failed", error = %e, "argon2id hashing failed");
            None
        }
    }
}

/// Verify `plaintext` against a stored argon2id PHC string in constant time
/// (argon2's verifier compares the derived tag, not the strings).
fn verify_argon2id(plaintext: &str, stored_phc: &str) -> bool {
    let parsed = match PasswordHash::new(stored_phc) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(reason = "argon2_phc_parse_failed", error = %e, "stored argon2id PHC string is malformed");
            return false;
        }
    };
    argon2id()
        .verify_password(plaintext.as_bytes(), &parsed)
        .is_ok()
}

/// Opportunistically migrate a verified legacy account to argon2id: store the
/// new hash, flip `password_algo` to argon2id, and NULL the legacy column.
///
/// Best-effort: any failure (hashing or DB) is logged per the negative-logging
/// convention and swallowed. The caller has already verified the credential, so
/// a failed migration must not fail the login.
async fn migrate_to_argon2id(db: &PgPool, account_id: i32, plaintext: &str) {
    let phc = match hash_argon2id(plaintext) {
        Some(h) => h,
        None => return, // already logged in hash_argon2id
    };

    let result = sqlx::query(
        "UPDATE account \
         SET password_hash_v2 = $1, password_algo = $2, password = NULL \
         WHERE account_id = $3",
    )
    .bind(&phc)
    .bind(ALGO_ARGON2ID)
    .bind(account_id)
    .execute(db)
    .await;

    match result {
        Ok(r) if r.rows_affected() == 1 => {
            tracing::info!(account_id, "migrated account to argon2id on login");
        }
        Ok(r) => {
            tracing::warn!(
                account_id,
                rows_affected = r.rows_affected(),
                reason = "argon2_migration_no_row",
                "argon2id migration UPDATE matched an unexpected row count"
            );
        }
        Err(e) => {
            tracing::warn!(
                account_id,
                error = %e,
                reason = "argon2_migration_db_error",
                "argon2id migration UPDATE failed; login still succeeds on legacy hash"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_credential (no DB) ─────────────────────────────────────

    #[test]
    fn classify_40_hex_is_legacy_over_http() {
        let raw = "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3";
        assert_eq!(
            classify_credential(raw, false),
            Ok(ClientCredential::LegacySha1Hex(raw)),
            "40-char hex must classify as legacy SHA-1 even over plain HTTP"
        );
    }

    #[test]
    fn classify_plaintext_over_tls_ok() {
        assert_eq!(
            classify_credential("hunter2", true),
            Ok(ClientCredential::Plaintext("hunter2")),
            "plaintext over TLS must be accepted"
        );
    }

    /// The security-critical gate: a plaintext password offered over plain
    /// HTTP must be rejected. Reverting the `over_tls` check trips this.
    #[test]
    fn classify_plaintext_over_http_rejected() {
        assert_eq!(
            classify_credential("hunter2", false),
            Err(CredentialGateError::PlaintextRequiresTls),
            "plaintext over plain HTTP must be rejected"
        );
    }

    #[test]
    fn classify_empty_plaintext_rejected_even_over_tls() {
        assert_eq!(
            classify_credential("", true),
            Err(CredentialGateError::PlaintextLength),
        );
    }

    #[test]
    fn classify_overlong_plaintext_rejected() {
        let long = "x".repeat(MAX_PLAINTEXT_LEN + 1);
        assert_eq!(
            classify_credential(&long, true),
            Err(CredentialGateError::PlaintextLength),
        );
        // Exactly at the bound is allowed.
        let at_bound = "x".repeat(MAX_PLAINTEXT_LEN);
        assert!(matches!(
            classify_credential(&at_bound, true),
            Ok(ClientCredential::Plaintext(_))
        ));
    }

    /// A 39-char hex string isn't a legacy hash; over HTTP it's plaintext and
    /// must be gated. Pins the exact 40-char boundary.
    #[test]
    fn classify_39_hex_is_plaintext_not_legacy() {
        let raw = "A94A8FE5CCB19BA61C4C0873D391E987982FBBD"; // 39 chars
        assert_eq!(
            classify_credential(raw, false),
            Err(CredentialGateError::PlaintextRequiresTls),
            "sub-40-char hex is plaintext, not a legacy hash"
        );
    }

    // ── argon2id hash/verify round trip (no DB) ─────────────────────────

    #[test]
    fn argon2id_hash_then_verify_roundtrip() {
        let phc = hash_argon2id("correct horse battery staple").expect("hash must succeed");
        assert!(
            phc.starts_with("$argon2id$"),
            "stored hash must be an argon2id PHC string, got {phc}"
        );
        assert!(verify_argon2id("correct horse battery staple", &phc));
        assert!(!verify_argon2id("wrong password", &phc));
    }

    #[test]
    fn sha1_upper_hex_matches_known_vector() {
        // SHA-1("test") = a94a8fe5ccb19ba61c4c0873d391e987982fbbd3
        assert_eq!(
            sha1_upper_hex("test"),
            "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3"
        );
    }

    /// A corrupt stored PHC string must be rejected (verification fails),
    /// never panic. Guards the parse-error branch in `verify_argon2id`.
    #[test]
    fn verify_argon2id_rejects_malformed_phc() {
        assert!(
            !verify_argon2id("anything", "not-a-valid-phc-string"),
            "a malformed stored hash must fail verification, not panic"
        );
        // A syntactically PHC-shaped but non-argon2 hash is also rejected.
        assert!(!verify_argon2id("anything", "$1$abc$def"));
    }

    // ── Live-DB credential + migration guards ───────────────────────────
    //
    // These exercise `validate_credentials` against a real `account` table
    // (the CI live-DB job loads `db/database.sql`). They self-skip without
    // `DATABASE_URL`. Sentinel account ids live in the crate's `0x7000_1B00`
    // window (fits in i32); cleanup deletes by exact id, never by range.

    use crate::test_support::require_db_or_skip;

    /// Sentinel base for the credential live-DB tests. Next free slot after
    /// character-create's `0x7000_1A00` window. Each test offsets a small
    /// constant so concurrent runs don't collide.
    const TEST_BASE: i32 = 0x7000_1B00;

    async fn cleanup(pool: &PgPool, account_id: i32) {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    /// Insert a legacy (algo 1) account whose `password` is the uppercase-hex
    /// SHA-1 of `plaintext`.
    async fn insert_legacy(pool: &PgPool, account_id: i32, name: &str, plaintext: &str) {
        cleanup(pool, account_id).await;
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
             VALUES ($1, $2, $3, 1, 2, true)",
        )
        .bind(account_id)
        .bind(name)
        .bind(sha1_upper_hex(plaintext))
        .execute(pool)
        .await
        .expect("insert legacy account");
    }

    /// Insert an argon2id (algo 2) account whose `password_hash_v2` is the PHC
    /// hash of `plaintext` and whose legacy `password` column is NULL.
    async fn insert_argon2(pool: &PgPool, account_id: i32, name: &str, plaintext: &str) {
        cleanup(pool, account_id).await;
        let phc = hash_argon2id(plaintext).expect("hash for fixture");
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password, password_hash_v2, password_algo, accesslevel, enabled) \
             VALUES ($1, $2, NULL, $3, 2, 2, true)",
        )
        .bind(account_id)
        .bind(name)
        .bind(phc)
        .execute(pool)
        .await
        .expect("insert argon2 account");
    }

    /// Legacy SHA-1 hash login still succeeds, and the row stays legacy
    /// (algo 1, password intact) — the unpatched-client path must never
    /// migrate. Reverting the `(1, LegacySha1Hex)` arm trips this.
    #[tokio::test]
    async fn legacy_hash_login_succeeds_without_migration() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 1;
        let name = format!("cred-legacy-{id}");
        insert_legacy(&pool, id, &name, "secretpw").await;

        let hash = sha1_upper_hex("secretpw");
        let res = validate_credentials(&pool, &name, ClientCredential::LegacySha1Hex(&hash)).await;
        assert!(res.is_ok(), "legacy hash login must succeed");

        let (algo, pw, v2): (i16, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT password_algo, password, password_hash_v2 FROM account WHERE account_id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(algo, 1, "legacy hash login must NOT migrate the row");
        assert!(pw.is_some(), "legacy password column must stay populated");
        assert!(v2.is_none(), "no argon2id hash must be written");

        cleanup(&pool, id).await;
    }

    /// Plaintext login against a legacy account succeeds AND opportunistically
    /// migrates: row flips to algo 2, password_hash_v2 populated, password
    /// NULLed. Reverting the `migrate_to_argon2id` call trips this.
    #[tokio::test]
    async fn plaintext_login_migrates_legacy_account() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 2;
        let name = format!("cred-migrate-{id}");
        insert_legacy(&pool, id, &name, "migrate-me").await;

        let res =
            validate_credentials(&pool, &name, ClientCredential::Plaintext("migrate-me")).await;
        assert!(res.is_ok(), "correct plaintext against legacy must succeed");

        let (algo, pw, v2): (i16, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT password_algo, password, password_hash_v2 FROM account WHERE account_id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(algo, 2, "login must migrate the row to argon2id (algo 2)");
        assert!(
            pw.is_none(),
            "legacy password column must be NULLed on migration"
        );
        let v2 = v2.expect("argon2id hash must be written");
        assert!(
            v2.starts_with("$argon2id$"),
            "migrated hash must be an argon2id PHC string, got {v2}"
        );
        // And the migrated hash actually verifies the same plaintext.
        assert!(verify_argon2id("migrate-me", &v2));

        cleanup(&pool, id).await;
    }

    /// argon2id account: correct plaintext succeeds; wrong plaintext yields
    /// InvalidCredentials.
    #[tokio::test]
    async fn argon2_account_accepts_correct_rejects_wrong_plaintext() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 3;
        let name = format!("cred-argon-{id}");
        insert_argon2(&pool, id, &name, "right-password").await;

        let ok =
            validate_credentials(&pool, &name, ClientCredential::Plaintext("right-password")).await;
        assert!(
            ok.is_ok(),
            "correct plaintext against argon2id must succeed"
        );

        let bad =
            validate_credentials(&pool, &name, ClientCredential::Plaintext("wrong-password")).await;
        assert!(
            matches!(bad, Err(AuthCredError::InvalidCredentials)),
            "wrong plaintext must be InvalidCredentials"
        );

        cleanup(&pool, id).await;
    }

    /// argon2id account rejects a 40-hex-shaped (legacy) credential: a stored
    /// argon2id hash can't be verified from a client SHA-1 hash, so the
    /// unpatched client can no longer log into a migrated account.
    #[tokio::test]
    async fn argon2_account_rejects_legacy_hash_shape() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 4;
        let name = format!("cred-argon-legacy-{id}");
        insert_argon2(&pool, id, &name, "right-password").await;

        // Even the *correct* SHA-1 hash of the password must be rejected for
        // an argon2id account — there is no way to verify it.
        let hash = sha1_upper_hex("right-password");
        let res = validate_credentials(&pool, &name, ClientCredential::LegacySha1Hex(&hash)).await;
        assert!(
            matches!(res, Err(AuthCredError::InvalidCredentials)),
            "argon2id account must reject a legacy-hash-shaped credential"
        );

        cleanup(&pool, id).await;
    }

    /// Missing account yields InvalidCredentials (not a distinct error) so
    /// account existence is never revealed.
    #[tokio::test]
    async fn unknown_account_is_invalid_credentials() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 5;
        cleanup(&pool, id).await; // ensure absent
        let res = validate_credentials(
            &pool,
            "cred-nonexistent-account-xyz",
            ClientCredential::Plaintext("whatever"),
        )
        .await;
        assert!(
            matches!(res, Err(AuthCredError::InvalidCredentials)),
            "unknown account must be InvalidCredentials, not a distinct error"
        );
    }

    /// An account flagged argon2id (algo 2) but whose `password_hash_v2` is
    /// NULL must fail closed (InvalidCredentials), never panic. Guards the
    /// `argon2_hash_missing` branch — a corrupt/half-migrated row.
    #[tokio::test]
    async fn argon2_account_with_null_hash_is_invalid() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 6;
        let name = format!("cred-argon-nullhash-{id}");
        cleanup(&pool, id).await;
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password, password_hash_v2, password_algo, accesslevel, enabled) \
             VALUES ($1, $2, NULL, NULL, 2, 2, true)",
        )
        .bind(id)
        .bind(&name)
        .execute(&pool)
        .await
        .expect("insert argon2 account with NULL hash");

        let res = validate_credentials(&pool, &name, ClientCredential::Plaintext("whatever")).await;
        assert!(
            matches!(res, Err(AuthCredError::InvalidCredentials)),
            "argon2id account with NULL hash must fail closed"
        );

        cleanup(&pool, id).await;
    }

    /// An account with an unrecognised `password_algo` must fail closed, never
    /// authenticate. Guards the catch-all arm against a future/garbage algo.
    #[tokio::test]
    async fn unknown_password_algo_is_invalid() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 7;
        let name = format!("cred-badalgo-{id}");
        cleanup(&pool, id).await;
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
             VALUES ($1, $2, $3, 99, 2, true)",
        )
        .bind(id)
        .bind(&name)
        .bind(sha1_upper_hex("whatever"))
        .execute(&pool)
        .await
        .expect("insert account with unknown algo");

        // Even the matching SHA-1 hash must not authenticate an unknown-algo row.
        let hash = sha1_upper_hex("whatever");
        let res = validate_credentials(&pool, &name, ClientCredential::LegacySha1Hex(&hash)).await;
        assert!(
            matches!(res, Err(AuthCredError::InvalidCredentials)),
            "unrecognised password_algo must fail closed"
        );

        cleanup(&pool, id).await;
    }
}

//! argon2id password hashing for stored credentials: the OWASP parameter set,
//! hashing a plaintext into a PHC string, constant-time verification, and the
//! best-effort on-login migration of a verified legacy SHA-1 account.
//!
//! Split out of `credentials.rs`, which classifies the client credential and
//! decides which of these to call.

use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use sqlx::PgPool;

use super::credentials::ALGO_ARGON2ID;

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
///
/// `pub(super)` so the `credentials` tests and the TLS smoke can seed
/// argon2id fixture accounts.
pub(super) fn hash_argon2id(plaintext: &str) -> Option<String> {
    // `hash_password` draws a fresh 16-byte salt from the OS RNG — the same
    // length `SaltString::generate` produced under argon2 0.5, so new PHC
    // strings keep the `$argon2id$v=19$m=65536,t=3,p=1$<22-char salt>$` shape.
    match argon2id().hash_password(plaintext.as_bytes()) {
        Ok(hash) => Some(hash.to_string()),
        Err(e) => {
            tracing::error!(reason = "argon2_hash_failed", error = %e, "argon2id hashing failed");
            None
        }
    }
}

/// Verify `plaintext` against a stored argon2id PHC string in constant time
/// (argon2's verifier compares the derived tag, not the strings).
pub(super) fn verify_argon2id(plaintext: &str, stored_phc: &str) -> bool {
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
pub(super) async fn migrate_to_argon2id(
    db: &PgPool,
    account_id: i32,
    account_name: &str,
    plaintext: &str,
) {
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
            tracing::info!(
                account_id,
                account_name,
                "migrated account to argon2id on login"
            );
        }
        Ok(r) => {
            tracing::warn!(
                account_id,
                account_name,
                rows_affected = r.rows_affected(),
                reason = "argon2_migration_no_row",
                "argon2id migration UPDATE matched an unexpected row count"
            );
        }
        Err(e) => {
            tracing::warn!(
                account_id,
                account_name,
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

    /// Crate-upgrade guard: every stored `password_hash_v2` row was minted by
    /// argon2 0.5.3. This PHC string came from 0.5.3's
    /// `SaltString::generate` + `hash_password` with the exact `argon2id()`
    /// params. An argon2/password-hash bump that changes salt decoding or PHC
    /// param parsing would lock every migrated account out — and trips this.
    #[test]
    fn verify_argon2id_accepts_hash_minted_by_argon2_0_5() {
        const PHC_FROM_ARGON2_0_5: &str = "$argon2id$v=19$m=65536,t=3,p=1$\
             /ldZYr+2G7/DWlGkh6t28A$PiniNilfh8g0JVhbVhhR5cCEVFi4fqrvx5SCF0G2Szc";
        assert!(
            verify_argon2id("cimmeria-argon2-0.5-fixture", PHC_FROM_ARGON2_0_5),
            "a hash minted by argon2 0.5 must still verify"
        );
        assert!(!verify_argon2id("wrong password", PHC_FROM_ARGON2_0_5));
    }

    /// New hashes keep the stored shape: argon2id v19, the OWASP params, a
    /// 16-byte salt (22 B64 chars) and a 32-byte tag (43 B64 chars).
    #[test]
    fn hash_argon2id_emits_owasp_params_and_16_byte_salt() {
        let phc = hash_argon2id("shape-check").expect("hash must succeed");
        let fields: Vec<&str> = phc.split('$').collect();
        assert_eq!(
            &fields[..4],
            &["", "argon2id", "v=19", "m=65536,t=3,p=1"],
            "got {phc}"
        );
        assert_eq!(fields[4].len(), 22, "salt must be 16 bytes, got {phc}");
        assert_eq!(fields[5].len(), 43, "tag must be 32 bytes, got {phc}");
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
}

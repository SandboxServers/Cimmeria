//! Refresh tokens (contract C6, D-CT14): issuing them, rotating each one on
//! use with reuse detection, and revoking them. Only the SHA-256 of a token is
//! stored (C5, R7), so a database read yields nothing that can be presented.
//!
//! The database decides every time check against the caller's `now`, so the
//! tests can move the clock without a stub. Timestamps cross as RFC 3339 text
//! cast to `timestamptz`.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rand::RngExt;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};

use super::RepoError;

/// The idle limit: a token unused for this long (`last_used_at`) is expired.
/// Provisional (D-CT14).
pub const REFRESH_IDLE: Duration = Duration::days(30);

/// The family limit: a family ends this long after its first token was issued
/// (`family_expires_at`), however often it rotates. Provisional (D-CT14).
pub const REFRESH_FAMILY: Duration = Duration::days(180);

/// The outcome of presenting a refresh token to [`rotate`]. It has no `Debug`
/// impl, so a token never reaches a log through a derived format.
pub enum Rotation {
    /// The token was live. It is now replaced, and `token` continues the family.
    Rotated {
        /// The account the family belongs to.
        account_id: u32,
        /// The family the new token belongs to.
        family_id: String,
        /// The new refresh token, for the client to keep.
        token: String,
    },
    /// The token was already revoked or replaced, so the whole family is revoked.
    Reused {
        /// The account the family belongs to.
        account_id: u32,
        /// The family that was revoked.
        family_id: String,
    },
    /// The token was idle too long or its family is past its limit. The family
    /// is revoked.
    Expired,
    /// No row holds this token.
    Unknown,
}

/// The state of one presented row: `account_id`, `family_id`, whether it is
/// spent (revoked or replaced), whether it is still inside the idle limit and
/// whether its family is still inside the family limit.
type StateRow = (i32, String, bool, bool, bool);

/// A fresh refresh token and the SHA-256 of its text. The token is 32 random
/// bytes in base64url without padding.
pub fn new_token() -> (String, [u8; 32]) {
    let mut rng = rand::rng();
    let bytes: [u8; 32] = std::array::from_fn(|_| rng.random());
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let hash = hash_of(&token);
    (token, hash)
}

/// The SHA-256 of a token's text: the only form the database keeps.
fn hash_of(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// A new family id: 16 random bytes as 32 lowercase hex characters.
fn new_family_id() -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 16] = std::array::from_fn(|_| rng.random());
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A timestamp as the RFC 3339 text the queries cast to `timestamptz`, to the
/// microsecond the database keeps.
fn micros(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(SecondsFormat::Micros, true)
}

/// Issues a refresh token for `account_id` and returns it with its family id.
///
/// With `family` `None` the token starts a new family, which ends
/// [`REFRESH_FAMILY`] after `now`. With `Some(id)` it joins that family and
/// takes the family's existing expiry. A family with no rows yet, given as
/// `Some`, starts at `now` as a new family would.
pub async fn issue(
    pool: &PgPool,
    account_id: u32,
    family: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(String, String), RepoError> {
    let (token, hash) = new_token();
    let family_id = family.map_or_else(new_family_id, str::to_owned);
    let now_ts = micros(now);
    let family_ends = micros(now + REFRESH_FAMILY);
    sqlx::query(
        "INSERT INTO auth_refresh_token \
         (token_hash, family_id, account_id, issued_at, last_used_at, family_expires_at) \
         VALUES ($1, $2, $3, $4::timestamptz, $4::timestamptz, \
           COALESCE((SELECT MIN(family_expires_at) FROM auth_refresh_token WHERE family_id = $2), \
                    $5::timestamptz))",
    )
    .bind(&hash[..])
    .bind(&family_id)
    .bind(account_id as i32)
    .bind(&now_ts)
    .bind(&family_ends)
    .execute(pool)
    .await?;
    Ok((token, family_id))
}

/// Presents a refresh token and rotates it in one transaction.
///
/// The row is read `FOR UPDATE`. A spent row (revoked or replaced) is
/// [`Rotation::Reused`] and revokes its whole family. A row past the idle or
/// family limit is [`Rotation::Expired`] and revokes its family. A live row is
/// replaced by [`Rotation::Rotated`]: the old row gets `replaced_by` and
/// `revoked_at`, and the new row joins the same family with the same expiry.
/// A token no row holds is [`Rotation::Unknown`].
pub async fn rotate(
    pool: &PgPool,
    presented: &str,
    now: DateTime<Utc>,
) -> Result<Rotation, RepoError> {
    let old_hash = hash_of(presented);
    let now_ts = micros(now);
    let mut tx = pool.begin().await?;
    let row: Option<StateRow> = sqlx::query_as(
        "SELECT account_id, family_id, \
           revoked_at IS NOT NULL OR replaced_by IS NOT NULL, \
           last_used_at + make_interval(secs => $2::float8) > $3::timestamptz, \
           family_expires_at > $3::timestamptz \
         FROM auth_refresh_token WHERE token_hash = $1 FOR UPDATE",
    )
    .bind(&old_hash[..])
    .bind(REFRESH_IDLE.num_seconds() as f64)
    .bind(&now_ts)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((account, family_id, spent, idle_ok, family_ok)) = row else {
        return Ok(Rotation::Unknown);
    };
    let account_id = account as u32;

    if spent {
        revoke_family_on(&mut tx, &family_id, &now_ts).await?;
        tx.commit().await?;
        return Ok(Rotation::Reused {
            account_id,
            family_id,
        });
    }
    if !idle_ok || !family_ok {
        revoke_family_on(&mut tx, &family_id, &now_ts).await?;
        tx.commit().await?;
        return Ok(Rotation::Expired);
    }

    let (token, new_hash) = new_token();
    sqlx::query(
        "UPDATE auth_refresh_token SET replaced_by = $2, revoked_at = $3::timestamptz, \
         last_used_at = $3::timestamptz WHERE token_hash = $1",
    )
    .bind(&old_hash[..])
    .bind(&new_hash[..])
    .bind(&now_ts)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO auth_refresh_token \
         (token_hash, family_id, account_id, issued_at, last_used_at, family_expires_at) \
         SELECT $1::bytea, family_id, account_id, $2::timestamptz, $2::timestamptz, family_expires_at \
         FROM auth_refresh_token WHERE token_hash = $3::bytea",
    )
    .bind(&new_hash[..])
    .bind(&now_ts)
    .bind(&old_hash[..])
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Rotation::Rotated {
        account_id,
        family_id,
        token,
    })
}

/// Revokes every live token of one family. Rows already revoked keep their
/// time.
///
/// The live rows are locked first, in their own statement. That statement
/// waits for an in-flight [`rotate`], so the revoke below runs on a fresh
/// snapshot and also sees the token the rotation inserted.
pub async fn revoke_family(
    pool: &PgPool,
    family_id: &str,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "SELECT 1 FROM auth_refresh_token \
         WHERE family_id = $1 AND revoked_at IS NULL FOR UPDATE",
    )
    .bind(family_id)
    .fetch_all(&mut *tx)
    .await?;
    revoke_family_on(&mut tx, family_id, &micros(now)).await?;
    tx.commit().await?;
    Ok(())
}

/// Revokes every live token of an account, in every family. Used when the
/// account is disabled (C6). Locks the live rows first, as [`revoke_family`]
/// does, so a rotation in flight cannot leave a live token behind.
pub async fn revoke_account(
    pool: &PgPool,
    account_id: u32,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "SELECT 1 FROM auth_refresh_token \
         WHERE account_id = $1 AND revoked_at IS NULL FOR UPDATE",
    )
    .bind(account_id as i32)
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE auth_refresh_token SET revoked_at = $2::timestamptz \
         WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(account_id as i32)
    .bind(micros(now))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Logout (C6): revokes the family of a presented token. A token no row holds
/// does nothing.
pub async fn revoke_token(
    pool: &PgPool,
    presented: &str,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT family_id FROM auth_refresh_token WHERE token_hash = $1")
            .bind(&hash_of(presented)[..])
            .fetch_optional(pool)
            .await?;
    if let Some((family_id,)) = row {
        revoke_family(pool, &family_id, now).await?;
    }
    Ok(())
}

/// Deletes the rows no check can accept again: a row whose family has passed
/// `family_expires_at`, or whose token has been idle longer than
/// [`REFRESH_IDLE`]. Returns how many rows went. An expired or unknown token
/// already answers `invalid_grant`, so this does not weaken reuse detection for
/// rows past their life.
pub async fn purge_expired(pool: &PgPool) -> Result<u64, RepoError> {
    let done = sqlx::query(
        "DELETE FROM auth_refresh_token WHERE family_expires_at < now() \
         OR last_used_at < now() - make_interval(secs => $1::float8)",
    )
    .bind(REFRESH_IDLE.num_seconds() as f64)
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

/// Revokes the live rows of one family on the given connection, so it commits
/// with the caller's transaction.
async fn revoke_family_on(
    conn: &mut PgConnection,
    family_id: &str,
    now_ts: &str,
) -> Result<(), RepoError> {
    sqlx::query(
        "UPDATE auth_refresh_token SET revoked_at = $2::timestamptz \
         WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(family_id)
    .bind(now_ts)
    .execute(conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use cimmeria_test_support::require_db_or_skip;
    use sqlx::PgPool;

    use super::*;

    const ID_ROTATE: u32 = 2_000_301;
    const ID_REUSE: u32 = 2_000_302;
    const ID_EXPIRE: u32 = 2_000_303;
    const ID_HASH: u32 = 2_000_304;
    const ID_ACCOUNT: u32 = 2_000_305;
    const ID_LOGOUT: u32 = 2_000_306;
    const ID_CONCURRENT: u32 = 2_000_307;
    const ID_PURGE: u32 = 2_000_308;

    /// The fixed clock every test starts from.
    fn base() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 24, 12, 0, 0)
            .single()
            .expect("a valid date")
    }

    /// The clock `days` after [`base`].
    fn at(days: i64) -> DateTime<Utc> {
        base() + Duration::days(days)
    }

    /// Delete the sentinel account `account_id`. Its token rows cascade with it.
    async fn cleanup(pool: &PgPool, account_id: u32) {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id as i32)
            .execute(pool)
            .await;
    }

    /// Insert a sentinel account with no password, after clearing any old one.
    async fn insert_account(pool: &PgPool, account_id: u32) {
        cleanup(pool, account_id).await;
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
             VALUES ($1, $2, NULL, 1, 0, true)",
        )
        .bind(account_id as i32)
        .bind(format!("ct16_refresh_{account_id}"))
        .execute(pool)
        .await
        .expect("insert sentinel account");
    }

    /// The new token of a [`Rotation::Rotated`], or a panic naming the failure.
    fn rotated_token(outcome: Result<Rotation, RepoError>) -> String {
        match outcome.expect("rotate") {
            Rotation::Rotated { token, .. } => token,
            _ => panic!("a live token rotates"),
        }
    }

    #[test]
    fn new_token_is_43_base64url_characters_and_hashes_its_text() {
        let (token, hash) = new_token();
        assert_eq!(token.len(), 43, "32 bytes in base64url without padding");
        assert!(token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_eq!(hash, hash_of(&token));
        assert_ne!(new_token().0, token, "tokens are random");
    }

    #[tokio::test]
    async fn live_db_rotation_replaces_the_token() {
        let pool = require_db_or_skip!();
        insert_account(&pool, ID_ROTATE).await;
        let (old, family) = issue(&pool, ID_ROTATE, None, at(0)).await.expect("issue");
        let outcome = rotate(&pool, &old, at(1)).await;
        let row: Option<(bool, bool)> = sqlx::query_as(
            "SELECT replaced_by IS NOT NULL, revoked_at IS NOT NULL \
             FROM auth_refresh_token WHERE token_hash = $1",
        )
        .bind(&hash_of(&old)[..])
        .fetch_optional(&pool)
        .await
        .expect("read the old row");
        cleanup(&pool, ID_ROTATE).await;
        let Rotation::Rotated {
            account_id,
            family_id,
            token,
        } = outcome.expect("rotate")
        else {
            panic!("a live token rotates");
        };
        assert_eq!(account_id, ID_ROTATE);
        assert_eq!(family_id, family);
        assert_ne!(token, old);
        assert_eq!(row, Some((true, true)));
    }

    #[tokio::test]
    async fn live_db_reuse_revokes_the_family() {
        let pool = require_db_or_skip!();
        insert_account(&pool, ID_REUSE).await;
        let (a, family) = issue(&pool, ID_REUSE, None, at(0)).await.expect("issue");
        let b = rotated_token(rotate(&pool, &a, at(1)).await);
        let again = rotate(&pool, &a, at(2)).await;
        let with_b = rotate(&pool, &b, at(3)).await;
        cleanup(&pool, ID_REUSE).await;
        match again.expect("rotate the spent token") {
            Rotation::Reused {
                account_id,
                family_id,
            } => {
                assert_eq!(account_id, ID_REUSE);
                assert_eq!(family_id, family);
            }
            _ => panic!("a replaced token is reuse"),
        }
        assert!(
            matches!(with_b, Ok(Rotation::Reused { .. })),
            "the live sibling dies with the family"
        );
    }

    #[tokio::test]
    async fn live_db_idle_and_family_expiry() {
        let pool = require_db_or_skip!();
        insert_account(&pool, ID_EXPIRE).await;
        // Idle: unused for 31 days.
        let (idle, _) = issue(&pool, ID_EXPIRE, None, at(0)).await.expect("issue");
        let idle_outcome = rotate(&pool, &idle, at(31)).await;
        // Family: rotated every 25 days, so idle is fine at day 181, but the
        // family limit (180 days from day 0) has passed.
        let (mut token, _) = issue(&pool, ID_EXPIRE, None, at(0)).await.expect("issue");
        for day in (25..=175).step_by(25) {
            token = rotated_token(rotate(&pool, &token, at(day)).await);
        }
        let family_outcome = rotate(&pool, &token, at(181)).await;
        let after = rotate(&pool, &token, at(182)).await;
        cleanup(&pool, ID_EXPIRE).await;
        assert!(
            matches!(idle_outcome, Ok(Rotation::Expired)),
            "idle expires"
        );
        assert!(
            matches!(family_outcome, Ok(Rotation::Expired)),
            "the family limit expires"
        );
        assert!(
            matches!(after, Ok(Rotation::Reused { .. })),
            "an expired family stays revoked"
        );
    }

    /// **Purge (C12).** `purge_expired` deletes the row of a family past its
    /// limit and keeps a live row, and returns how many it deleted. With the
    /// DELETE replaced by a no-op the count is 0 and the dead row stays.
    #[tokio::test]
    async fn live_db_purge_expired_deletes_only_dead_rows() {
        let pool = require_db_or_skip!();
        insert_account(&pool, ID_PURGE).await;
        // Family limit passed 220 days ago, so the row is dead by `now()`.
        issue(&pool, ID_PURGE, None, Utc::now() - Duration::days(400))
            .await
            .expect("issue");
        let (live, _) = issue(&pool, ID_PURGE, None, Utc::now())
            .await
            .expect("issue");
        let deleted = purge_expired(&pool).await;
        let kept: Vec<(Vec<u8>,)> =
            sqlx::query_as("SELECT token_hash FROM auth_refresh_token WHERE account_id = $1")
                .bind(ID_PURGE as i32)
                .fetch_all(&pool)
                .await
                .expect("select the account's rows");
        cleanup(&pool, ID_PURGE).await;
        assert_eq!(deleted.expect("purge"), 1, "one dead row is deleted");
        assert_eq!(
            kept,
            vec![(hash_of(&live).to_vec(),)],
            "only the live row remains"
        );
    }

    #[tokio::test]
    async fn live_db_tokens_are_stored_hashed() {
        let pool = require_db_or_skip!();
        insert_account(&pool, ID_HASH).await;
        let (old, _) = issue(&pool, ID_HASH, None, at(0)).await.expect("issue");
        let new = rotated_token(rotate(&pool, &old, at(1)).await);
        let rows: Vec<(Vec<u8>, String, Option<Vec<u8>>)> = sqlx::query_as(
            "SELECT token_hash, family_id, replaced_by FROM auth_refresh_token \
             WHERE account_id = $1",
        )
        .bind(ID_HASH as i32)
        .fetch_all(&pool)
        .await
        .expect("read the token rows");
        cleanup(&pool, ID_HASH).await;
        assert_eq!(rows.len(), 2, "the old and the new row");
        for token in [old.as_str(), new.as_str()] {
            for (hash, family_id, replaced_by) in &rows {
                assert_ne!(hash.as_slice(), token.as_bytes(), "no bytea holds the text");
                assert!(!family_id.contains(token), "no text column holds the text");
                assert_ne!(replaced_by.as_deref(), Some(token.as_bytes()));
            }
        }
        for token in [&old, &new] {
            assert!(
                rows.iter()
                    .any(|(hash, _, _)| hash.as_slice() == hash_of(token)),
                "each row is keyed by the SHA-256 of its token"
            );
        }
    }

    #[tokio::test]
    async fn live_db_revoke_account_ends_every_family() {
        let pool = require_db_or_skip!();
        insert_account(&pool, ID_ACCOUNT).await;
        issue(&pool, ID_ACCOUNT, None, at(0))
            .await
            .expect("first family");
        issue(&pool, ID_ACCOUNT, None, at(0))
            .await
            .expect("second family");
        let revoked = revoke_account(&pool, ID_ACCOUNT, at(1)).await;
        let (live,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1 AND revoked_at IS NULL",
        )
        .bind(ID_ACCOUNT as i32)
        .fetch_one(&pool)
        .await
        .expect("count the live rows");
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT family_id) FROM auth_refresh_token WHERE account_id = $1",
        )
        .bind(ID_ACCOUNT as i32)
        .fetch_one(&pool)
        .await
        .expect("count the families");
        cleanup(&pool, ID_ACCOUNT).await;
        revoked.expect("revoke the account");
        assert_eq!(total, 2);
        assert_eq!(live, 0, "no family of a disabled account stays live");
    }

    #[tokio::test]
    async fn live_db_logout_revokes_its_family() {
        let pool = require_db_or_skip!();
        insert_account(&pool, ID_LOGOUT).await;
        let (token, _) = issue(&pool, ID_LOGOUT, None, at(0)).await.expect("issue");
        let logout = revoke_token(&pool, &token, at(1)).await;
        let after = rotate(&pool, &token, at(2)).await;
        cleanup(&pool, ID_LOGOUT).await;
        logout.expect("logout");
        assert!(
            matches!(after, Ok(Rotation::Reused { .. })),
            "the logged-out token and its family are revoked"
        );
    }

    #[tokio::test]
    async fn live_db_revoke_waits_for_an_in_flight_rotation() {
        let pool = require_db_or_skip!();
        insert_account(&pool, ID_CONCURRENT).await;
        let (token, family) = issue(&pool, ID_CONCURRENT, None, at(0))
            .await
            .expect("issue");
        let old_hash = hash_of(&token);
        let (_, new_hash) = new_token();
        // A rotation mid-flight: it holds the old row's lock and has inserted
        // the new row, but has not committed.
        let mut tx = pool.begin().await.expect("begin the rotation");
        sqlx::query("SELECT 1 FROM auth_refresh_token WHERE token_hash = $1 FOR UPDATE")
            .bind(&old_hash[..])
            .fetch_all(&mut *tx)
            .await
            .expect("lock the old row");
        sqlx::query(
            "UPDATE auth_refresh_token SET replaced_by = $1, revoked_at = $2::timestamptz \
             WHERE token_hash = $3",
        )
        .bind(&new_hash[..])
        .bind(micros(at(1)))
        .bind(&old_hash[..])
        .execute(&mut *tx)
        .await
        .expect("spend the old row");
        sqlx::query(
            "INSERT INTO auth_refresh_token \
             (token_hash, family_id, account_id, issued_at, last_used_at, family_expires_at) \
             SELECT $1, family_id, account_id, issued_at, last_used_at, family_expires_at \
             FROM auth_refresh_token WHERE token_hash = $2",
        )
        .bind(&new_hash[..])
        .bind(&old_hash[..])
        .execute(&mut *tx)
        .await
        .expect("insert the new row");
        // The revoke starts while the rotation is still open.
        let revoke = tokio::spawn({
            let pool = pool.clone();
            async move { revoke_family(&pool, &family, at(1)).await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        tx.commit().await.expect("commit the rotation");
        let revoked = revoke.await.expect("join the revoke");
        let (live,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1 AND revoked_at IS NULL",
        )
        .bind(ID_CONCURRENT as i32)
        .fetch_one(&pool)
        .await
        .expect("count the live rows");
        cleanup(&pool, ID_CONCURRENT).await;
        revoked.expect("revoke the family");
        assert_eq!(
            live, 0,
            "the token a rotation inserted mid-revoke is revoked"
        );
    }
}

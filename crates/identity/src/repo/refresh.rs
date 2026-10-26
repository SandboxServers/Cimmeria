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

/// The outcome of [`issue_bound`]. It has no `Debug` impl, for the same reason
/// as [`Rotation`].
pub enum Issued {
    /// A new token and its family id, `(refresh_token, family_id)`.
    Pair(String, String),
    /// The account is disabled, so nothing was issued.
    Disabled,
    /// The account is gone, or the identity is no longer linked to it.
    Unbound,
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
    let mut conn = pool.acquire().await?;
    issue_on(&mut conn, account_id, family, now).await
}

/// [`issue`] on the given connection, so it commits with the caller's
/// transaction.
pub async fn issue_on(
    conn: &mut PgConnection,
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
    .execute(conn)
    .await?;
    Ok((token, family_id))
}

/// Issues a new family for `account_id`, but only while the account is enabled
/// and `identity_id` is still linked to it (C16.2).
///
/// The account row is locked `FOR SHARE` for the whole transaction. An unlink
/// holds it `FOR UPDATE` across its delete, so an unlink in flight is waited for
/// and the link check then sees its result. A disabled account or an unlinked
/// identity issues nothing and returns [`Issued::Disabled`] or
/// [`Issued::Unbound`].
pub async fn issue_bound(
    pool: &PgPool,
    account_id: u32,
    identity_id: i64,
    now: DateTime<Utc>,
) -> Result<Issued, RepoError> {
    let account = account_id as i32;
    let mut tx = pool.begin().await?;
    let enabled: Option<(bool,)> =
        sqlx::query_as("SELECT enabled FROM account WHERE account_id = $1 FOR SHARE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((enabled,)) = enabled else {
        tx.rollback().await?;
        return Ok(Issued::Unbound);
    };
    if !enabled {
        tx.rollback().await?;
        return Ok(Issued::Disabled);
    }
    let linked: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM account_identity WHERE identity_id = $2 AND account_id = $1)",
    )
    .bind(account)
    .bind(identity_id)
    .fetch_one(&mut *tx)
    .await?;
    if !linked {
        tx.rollback().await?;
        return Ok(Issued::Unbound);
    }
    let (token, family_id) = issue_on(&mut tx, account_id, None, now).await?;
    tx.commit().await?;
    Ok(Issued::Pair(token, family_id))
}

/// Presents a refresh token and rotates it in one transaction.
///
/// The row is read `FOR UPDATE`. A spent row (revoked or replaced) is
/// [`Rotation::Reused`] and revokes its whole family. A row past the idle or
/// family limit is [`Rotation::Expired`] and revokes its family. A live row is
/// replaced by [`Rotation::Rotated`]: the old row gets `replaced_by` and
/// `revoked_at`, and the new row joins the same family with the same expiry.
/// A token no row holds is [`Rotation::Unknown`].
///
/// The family lock is taken before the row is read, so the read sees any
/// successor another operation on the family inserted and committed (C16.1).
pub async fn rotate(
    pool: &PgPool,
    presented: &str,
    now: DateTime<Utc>,
) -> Result<Rotation, RepoError> {
    let old_hash = hash_of(presented);
    let now_ts = micros(now);
    let mut tx = pool.begin().await?;
    let family: Option<(String,)> =
        sqlx::query_as("SELECT family_id FROM auth_refresh_token WHERE token_hash = $1")
            .bind(&old_hash[..])
            .fetch_optional(&mut *tx)
            .await?;
    let Some((family,)) = family else {
        return Ok(Rotation::Unknown);
    };
    family_lock(&mut tx, &family).await?;
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
/// The family lock is taken first (see [`family_lock`]): a rotation or a
/// successor insert in flight finishes before the revoke reads the rows, so the
/// revoke also sees the token it inserted.
pub async fn revoke_family(
    pool: &PgPool,
    family_id: &str,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let mut tx = pool.begin().await?;
    revoke_family_on(&mut tx, family_id, &micros(now)).await?;
    tx.commit().await?;
    Ok(())
}

/// Revokes every live token of an account, in every family. Used when the
/// account is disabled (C6). Each family is locked first, as [`revoke_family`]
/// does, so a rotation in flight cannot leave a live token behind.
pub async fn revoke_account(
    pool: &PgPool,
    account_id: u32,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let mut tx = pool.begin().await?;
    revoke_account_on(&mut tx, account_id, None, now).await?;
    tx.commit().await?;
    Ok(())
}

/// Revokes every live token of an account except those of `keep_family`, the
/// caller's own family (client-tls-oidc D1). Each family is locked first, as
/// [`revoke_account`] does.
pub async fn revoke_account_except(
    pool: &PgPool,
    account_id: u32,
    keep_family: &str,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let mut tx = pool.begin().await?;
    revoke_account_on(&mut tx, account_id, Some(keep_family), now).await?;
    tx.commit().await?;
    Ok(())
}

/// Revokes every live token of an account on the given connection, in every
/// family, or in every family but `keep_family` when one is given. Returns the
/// families it revoked, sorted and without repeats.
///
/// The families are listed, then each is locked in family-id order, then the
/// update runs. The lock is per family, not per account, so families do not
/// queue behind each other, and a successor inserted into a listed family waits
/// on that family's lock before the update runs.
pub async fn revoke_account_on(
    conn: &mut PgConnection,
    account_id: u32,
    keep_family: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Vec<String>, RepoError> {
    let families: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT family_id FROM auth_refresh_token \
         WHERE account_id = $1 AND ($2::text IS NULL OR family_id <> $2) \
         AND revoked_at IS NULL ORDER BY family_id",
    )
    .bind(account_id as i32)
    .bind(keep_family)
    .fetch_all(&mut *conn)
    .await?;
    for (family_id,) in &families {
        family_lock(conn, family_id).await?;
    }
    let revoked: Vec<(String,)> = sqlx::query_as(
        "UPDATE auth_refresh_token SET revoked_at = $3::timestamptz \
         WHERE account_id = $1 AND ($2::text IS NULL OR family_id <> $2) \
         AND revoked_at IS NULL RETURNING family_id",
    )
    .bind(account_id as i32)
    .bind(keep_family)
    .bind(micros(now))
    .fetch_all(conn)
    .await?;
    let mut ids: Vec<String> = revoked.into_iter().map(|(family_id,)| family_id).collect();
    ids.sort();
    ids.dedup();
    Ok(ids)
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

/// Deletes the rows of each family that has passed `family_expires_at`, or
/// whose every token has been idle longer than [`REFRESH_IDLE`]. A live family
/// keeps all its rows, spent ones too, so a replayed token still answers
/// `Reused`. Returns how many rows went. `now` is the caller's clock, as in
/// [`rotate`].
pub async fn purge_expired(pool: &PgPool, now: DateTime<Utc>) -> Result<u64, RepoError> {
    let done = sqlx::query(
        "DELETE FROM auth_refresh_token t WHERE t.family_expires_at < $2::timestamptz \
         OR NOT EXISTS (SELECT 1 FROM auth_refresh_token s \
           WHERE s.family_id = t.family_id \
           AND s.last_used_at + make_interval(secs => $1::float8) >= $2::timestamptz)",
    )
    .bind(REFRESH_IDLE.num_seconds() as f64)
    .bind(micros(now))
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

/// Takes the transaction-scoped advisory lock of one refresh family, released
/// at commit or rollback. Every operation that reads or writes a family's rows
/// takes it first. The lock is per family, not per account, because the
/// hazard is local to a family: a successor inserted between two statements of
/// another operation on that family (C16.1). Each family's lock is keyed by its
/// id, so unrelated families never wait on each other.
async fn family_lock(conn: &mut PgConnection, family_id: &str) -> Result<(), RepoError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('cimmeria.refresh_family:' || $1, 0))",
    )
    .bind(family_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// Revokes the live rows of one family on the given connection, so it commits
/// with the caller's transaction. Takes the family lock first (see
/// [`family_lock`]).
async fn revoke_family_on(
    conn: &mut PgConnection,
    family_id: &str,
    now_ts: &str,
) -> Result<(), RepoError> {
    family_lock(conn, family_id).await?;
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
#[path = "refresh_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "refresh_lock_tests_live_db.rs"]
mod lock_tests_live_db;

#[cfg(test)]
#[path = "refresh_bound_tests_live_db.rs"]
mod bound_tests_live_db;

//! Reusable persistence helpers shared by the Black Market state machine and
//! the expiry sweep: the clock and the cash adjustment. Item escrow is in
//! [`super::escrow`], the mail payout writer in [`super::payout_mail`].

/// Current unix epoch seconds, saturating into `i32` (matches the schema's
/// INTEGER time columns: `sent_time`, `created_at`, `expires_at`).
pub fn now_unix_secs() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .min(i32::MAX as u64) as i32
}

/// Why a cash adjustment was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum CashError {
    /// The player row does not exist.
    NoSuchPlayer,
    /// The adjustment would push the balance below zero (overdraw on a debit).
    InsufficientFunds,
    /// A credit would push the balance past the column's `i32` maximum.
    BalanceOverflow,
    /// Underlying DB failure.
    Db(String),
}

impl std::fmt::Display for CashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CashError::NoSuchPlayer => write!(f, "no such player"),
            CashError::InsufficientFunds => write!(f, "insufficient funds"),
            CashError::BalanceOverflow => write!(f, "balance overflow"),
            CashError::Db(e) => write!(f, "db error: {e}"),
        }
    }
}

/// Adjust a player's `naquadah` by `delta` (positive credit, negative debit),
/// rejecting any debit that would leave a negative balance. Returns the new
/// balance on success.
///
/// Both guards are enforced in SQL, in `bigint` arithmetic (`naquadah + $1`
/// between 0 and the `integer` maximum), so the check and the write are
/// atomic and a large credit is a named `BalanceOverflow` rather than an
/// integer-overflow error from Postgres — important for the bid-hold path where two
/// concurrent bids must not both pass a stale balance check. A `RETURNING`
/// miss is disambiguated from a missing row by a follow-up existence probe so
/// the caller gets `InsufficientFunds` vs `NoSuchPlayer` correctly.
///
/// NOTE: the executor must be re-borrowable for the probe, so this takes a
/// `&mut PgConnection`-style executor via two calls; callers pass `&mut *tx`.
pub async fn adjust_player_cash(
    conn: &mut sqlx::PgConnection,
    player_id: i32,
    delta: i64,
) -> Result<i64, CashError> {
    let updated = sqlx::query_scalar::<_, i64>(
        "UPDATE sgw_player SET naquadah = (naquadah::bigint + $1)::integer \
         WHERE player_id = $2 AND naquadah::bigint + $1 BETWEEN 0 AND 2147483647 \
         RETURNING naquadah::bigint",
    )
    .bind(delta)
    .bind(player_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| CashError::Db(e.to_string()))?;

    if let Some(balance) = updated {
        return Ok(balance);
    }

    // No row updated: either the player doesn't exist, or the guard rejected
    // the debit. Probe to tell the two apart.
    let exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| CashError::Db(e.to_string()))?
        .is_some();

    if exists && delta > 0 {
        Err(CashError::BalanceOverflow)
    } else if exists {
        Err(CashError::InsufficientFunds)
    } else {
        Err(CashError::NoSuchPlayer)
    }
}

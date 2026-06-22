//! Reusable persistence helpers shared by the Black Market state machine and
//! the expiry sweep — and intentionally generic enough to reuse elsewhere
//! (mail delivery, cash adjustment, item escrow/return).
//!
//! Every helper is generic over [`sqlx::PgExecutor`] so it runs against either a
//! bare `&PgPool` or `&mut Transaction`, letting the auction flows compose them
//! atomically inside a single transaction.

use sqlx::PgExecutor;

/// Current unix epoch seconds, saturating into `i32` (matches the schema's
/// INTEGER time columns: `sent_time`, `created_at`, `expires_at`).
pub fn now_unix_secs() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .min(i32::MAX as u64) as i32
}

/// Insert one `sgw_gate_mail` row delivering cash and/or an item to a recipient.
///
/// `cash` lands in the `cash` column (BIGINT); `item_id`, when `Some`, is the
/// `sgw_inventory.item_id` instance id attached to the mail. `flags` is written
/// verbatim. `sent_time` is stamped now; `read_time` is 0 (unread). Returns the
/// freshly-allocated `mail_id`.
///
/// Mirrors the column set the mail read/list path expects (see
/// `base/world_entry/methods/mail/`). `sender_id` is left NULL (system mail);
/// `sender_name` falls back to the column default.
pub async fn send_mail_to_player<'e, E>(
    exec: E,
    recipient_id: i32,
    cash: i64,
    item_id: Option<i32>,
    flags: i32,
    subject: &str,
    body: &str,
    sender_name: &str,
) -> Result<i32, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let now = now_unix_secs();
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, subject, message, cash, sent_time, read_time, \
             flags, item_id, sender_name) \
         VALUES ($1, NULL, $2, $3, $4, $5, 0, $6, $7, $8) \
         RETURNING mail_id",
    )
    .bind(recipient_id)
    .bind(subject)
    .bind(body)
    .bind(cash)
    .bind(now)
    .bind(flags)
    .bind(item_id)
    .bind(sender_name)
    .fetch_one(exec)
    .await
}

/// Why a cash adjustment was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum CashError {
    /// The player row does not exist.
    NoSuchPlayer,
    /// The adjustment would push the balance below zero (overdraw on a debit).
    InsufficientFunds,
    /// Underlying DB failure.
    Db(String),
}

impl std::fmt::Display for CashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CashError::NoSuchPlayer => write!(f, "no such player"),
            CashError::InsufficientFunds => write!(f, "insufficient funds"),
            CashError::Db(e) => write!(f, "db error: {e}"),
        }
    }
}

/// Adjust a player's `naquadah` by `delta` (positive credit, negative debit),
/// rejecting any debit that would leave a negative balance. Returns the new
/// balance on success.
///
/// The overdraw guard is enforced in SQL (`WHERE naquadah + $1 >= 0`) so the
/// check and the write are atomic — important for the bid-hold path where two
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
        "UPDATE sgw_player SET naquadah = naquadah + $1::int \
         WHERE player_id = $2 AND naquadah + $1::int >= 0 \
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

    if exists {
        Err(CashError::InsufficientFunds)
    } else {
        Err(CashError::NoSuchPlayer)
    }
}

/// Remove an item instance from a player's inventory so the auction can hold it
/// in escrow. Returns the deleted row's auctionable snapshot (def id, stack,
/// durability, charges) so the caller can record it on `sgw_auction`. Returns
/// `Ok(None)` if the player does not own that instance (validation failure).
///
/// This is a full DELETE of the instance row — auctions escrow whole instances,
/// not partial stacks (matching the client's single-item create flow).
pub async fn escrow_item<'e, E>(
    exec: E,
    player_id: i32,
    item_id: i32,
) -> Result<Option<EscrowedItem>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_as::<_, EscrowedItem>(
        "DELETE FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2 \
         RETURNING item_id, type_id AS item_def_id, stack_size, durability, charges",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(exec)
    .await
}

/// The snapshot of an escrowed inventory instance.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct EscrowedItem {
    pub item_id: i32,
    pub item_def_id: i32,
    pub stack_size: i32,
    pub durability: i32,
    pub charges: i32,
}

/// Return an escrowed item to a player's inventory by re-inserting an instance
/// in the default backpack container (container_id 0) at the next free slot.
///
/// Used by `cancelAuction` (seller reclaim) and the sweep's unsold path. The
/// instance gets a fresh `item_id` from the sequence — the original instance id
/// was consumed by the escrow DELETE. Returns the new instance id.
///
/// Slot placement mirrors `grant_item`: the new row lands at
/// `COALESCE(MAX(slot_id), -1) + 1` for this character's container 0, i.e. the
/// first slot past the current high-water mark. It must never insert at
/// `slot_id = -1` — that value is the inventory swap sentinel elsewhere in the
/// codebase, and a global row parked there breaks the inventory move/swap path.
pub async fn return_item<'e, E>(
    exec: E,
    player_id: i32,
    item_def_id: i32,
    stack_size: i32,
    durability: i32,
    charges: i32,
) -> Result<i32, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, \
             ammo_type, ammo_types, ammo, flags) \
         SELECT $1, ri.item_id, $2, \
                (SELECT COALESCE(MAX(inv.slot_id), -1) + 1 FROM sgw_inventory inv \
                  WHERE inv.character_id = $1 AND inv.container_id = 0), \
                0, false, $3, $4, \
                COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
                ri.ammo_types, ri.charges, 0 \
         FROM resources.items ri WHERE ri.item_id = $5 \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(stack_size)
    .bind(durability)
    .bind(charges)
    .bind(item_def_id)
    .fetch_one(exec)
    .await
}

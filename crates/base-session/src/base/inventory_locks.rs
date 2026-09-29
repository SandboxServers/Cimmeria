//! The advisory locks every inventory write takes on a player's inventory
//! before it locks any row: crafting, vendor, trade, mail, the Black Market
//! and the ammo reserve.
//!
//! It lived under `base::crafting` until crafting became a plugin (#962
//! step 5, `cimmeria-base-crafting`); the inventory code below the plugin
//! takes the same locks, so it stayed in the session layer.
//!
//! The shared inventory order is: every advisory lock first (the
//! player-wide key [`PLAYER_WIDE_LOCK`], then the per-(player, container)
//! keys in container order), then inventory rows, then the player row. The
//! move path, vendor purchase and crafting all take the player-wide key
//! first, so they serialize; trade and the generic grant take a bag key
//! before any row, so they wait on it instead of holding a row the crafting
//! write needs.

use sqlx::PgConnection;

/// The advisory-lock key the inventory move path takes for the whole
/// player before any per-bag lock.
pub const PLAYER_WIDE_LOCK: i32 = 0;

/// Take the player-wide lock, then one lock per bag in `bags` in container
/// order (sorted, each once), on `conn`.
pub async fn take_inventory_locks(
    conn: &mut PgConnection,
    player_id: i32,
    bags: &[i32],
) -> Result<(), sqlx::Error> {
    let mut bags = bags.to_vec();
    bags.sort_unstable();
    bags.dedup();
    for key in std::iter::once(PLAYER_WIDE_LOCK).chain(bags) {
        sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
            .bind(player_id)
            .bind(key)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

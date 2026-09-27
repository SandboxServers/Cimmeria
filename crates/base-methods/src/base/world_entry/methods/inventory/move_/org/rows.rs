//! The locked reads of a Team or Command vault move: the per-player
//! advisory locks, the dragged item's row and the target slot's occupant,
//! each `FOR UPDATE`, from whichever table holds it (bank-vault BV-07).

use super::super::{InventoryInstanceRow, MoveRequest, Occupant};
use super::{MoveTx, Side};

/// `pg_advisory_xact_lock(player_id, key)`: `key` 0 is the move lock, a
/// container id that container's lock (the personal move path's keys).
pub(super) async fn advisory(tx: &mut MoveTx, player_id: i32, key: i32) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(player_id)
        .bind(key)
        .execute(&mut **tx)
        .await
        .map(|_| ())
}

/// The dragged item, locked: the player's own row, else this
/// organization's vault row.
pub(super) async fn read_source(
    tx: &mut MoveTx,
    req: &MoveRequest,
    org_id: i32,
) -> Result<Option<(Side, InventoryInstanceRow)>, sqlx::Error> {
    let carried: Option<InventoryInstanceRow> = sqlx::query_as(
        "SELECT type_id, stack_size, container_id, slot_id, bound, durability, charges \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
    )
    .bind(req.player_id)
    .bind(req.item_id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = carried {
        return Ok(Some((Side::Carried, row)));
    }
    let vaulted: Option<InventoryInstanceRow> = sqlx::query_as(
        "SELECT type_id, stack_size, container_id, slot_id, bound, durability, charges \
         FROM sgw_organization_vault_items WHERE org_id = $1 AND item_id = $2 FOR UPDATE",
    )
    .bind(org_id)
    .bind(req.item_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(vaulted.map(|row| (Side::Vault, row)))
}

/// The row in the target slot, locked.
pub(super) async fn read_occupant(
    tx: &mut MoveTx,
    req: &MoveRequest,
    org_id: i32,
    side: Side,
) -> Result<Option<Occupant>, sqlx::Error> {
    let q = match side {
        Side::Carried => sqlx::query_as(
            "SELECT item_id, type_id, stack_size, bound, durability, charges FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = $2 AND slot_id = $3 AND item_id <> $4 \
             LIMIT 1 FOR UPDATE",
        )
        .bind(req.player_id),
        Side::Vault => sqlx::query_as(
            "SELECT item_id, type_id, stack_size, bound, durability, charges \
             FROM sgw_organization_vault_items \
             WHERE org_id = $1 AND container_id = $2 AND slot_id = $3 AND item_id <> $4 \
             LIMIT 1 FOR UPDATE",
        )
        .bind(org_id),
    };
    q.bind(req.target_container_id)
        .bind(req.target_slot_id)
        .bind(req.item_id)
        .fetch_optional(&mut **tx)
        .await
}

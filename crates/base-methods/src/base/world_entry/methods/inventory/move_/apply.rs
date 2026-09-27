//! The four writes a `moveItem` can make, each inside the caller's move
//! transaction: the whole stack into an empty slot, a split into an empty
//! slot, a merge into a same-type stack, and a swap with the occupant.
//!
//! Each returns `None` after logging when a statement fails or matches the
//! wrong number of rows; the caller then rolls back. None of them commits.

use sqlx::{Postgres, Transaction};

use super::{InventoryInstanceRow, Occupant};

type MoveTx = Transaction<'static, Postgres>;

/// Move the whole source stack into the empty target slot.
pub(super) async fn whole(
    tx: &mut MoveTx,
    player_id: i32,
    item_id: i32,
    target_container_id: i32,
    target_slot_id: i32,
) -> Option<()> {
    let result = sqlx::query(
        "UPDATE sgw_inventory SET container_id = $1, slot_id = $2 \
         WHERE character_id = $3 AND item_id = $4",
    )
    .bind(target_container_id)
    .bind(target_slot_id)
    .bind(player_id)
    .bind(item_id)
    .execute(&mut **tx)
    .await;

    match result {
        Ok(r) if r.rows_affected() == 1 => Some(()),
        Ok(_) => {
            tracing::warn!(player_id, item_id, "MoveInventoryItem: no rows updated");
            None
        }
        Err(e) => {
            tracing::error!(player_id, item_id, "MoveInventoryItem: update failed: {e}");
            None
        }
    }
}

/// Move `quantity` (less than the stack) into the empty target slot as a new
/// row. Returns the new row's id: for a split, the "moved" instance is the
/// new row in the target slot, not the decremented source stack, and that is
/// what `InventoryItemMoveApplied` consumers read.
pub(super) async fn split(
    tx: &mut MoveTx,
    player_id: i32,
    item_id: i32,
    quantity: i32,
    source: &InventoryInstanceRow,
    target_container_id: i32,
    target_slot_id: i32,
) -> Option<i32> {
    let update = sqlx::query(
        "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
         WHERE character_id = $2 AND item_id = $3 AND stack_size > $1",
    )
    .bind(quantity)
    .bind(player_id)
    .bind(item_id)
    .execute(&mut **tx)
    .await;

    let update_rows = match update {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                "MoveInventoryItem: split decrement failed: {e}"
            );
            return None;
        }
    };
    if update_rows != 1 {
        tracing::warn!(
            player_id,
            item_id,
            quantity,
            "MoveInventoryItem: split decrement matched 0 rows (concurrent modification?)"
        );
        return None;
    }

    let inserted: Result<Option<(i32,)>, _> = sqlx::query_as(
        "INSERT INTO sgw_inventory \
         (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(source.type_id)
    .bind(quantity)
    .bind(target_slot_id)
    .bind(target_container_id)
    .bind(source.bound)
    .bind(source.durability)
    .bind(source.charges)
    .fetch_optional(&mut **tx)
    .await;

    match inserted {
        Ok(Some((new_id,))) => Some(new_id),
        Ok(None) => {
            tracing::warn!(
                player_id,
                item_id,
                "MoveInventoryItem: split insert returned no row"
            );
            None
        }
        Err(e) => {
            tracing::error!(player_id, item_id, "MoveInventoryItem: split failed: {e}");
            None
        }
    }
}

/// Add `quantity` to the occupant, a stack of the same type with room for
/// it, and take it from the source: the source row is deleted when the whole
/// stack moves (legacy `Inventory.py:391-395`), and decremented otherwise.
/// Returns whether the source row was deleted.
pub(super) async fn merge(
    tx: &mut MoveTx,
    player_id: i32,
    item_id: i32,
    quantity: i32,
    source: &InventoryInstanceRow,
    occupant: &Occupant,
) -> Option<bool> {
    let source_deleted = quantity >= source.stack_size;
    let take = if source_deleted {
        sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1 AND item_id = $2")
            .bind(player_id)
            .bind(item_id)
            .execute(&mut **tx)
            .await
    } else {
        sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
             WHERE character_id = $2 AND item_id = $3 AND stack_size > $1",
        )
        .bind(quantity)
        .bind(player_id)
        .bind(item_id)
        .execute(&mut **tx)
        .await
    };
    match take {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(r) => {
            tracing::warn!(
                player_id,
                item_id,
                rows_affected = r.rows_affected(),
                expected = 1,
                "MoveInventoryItem: merge take from source matched no row"
            );
            return None;
        }
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                "MoveInventoryItem: merge take failed: {e}"
            );
            return None;
        }
    }

    let give = sqlx::query(
        "UPDATE sgw_inventory SET stack_size = stack_size + $1 \
         WHERE character_id = $2 AND item_id = $3",
    )
    .bind(quantity)
    .bind(player_id)
    .bind(occupant.item_id)
    .execute(&mut **tx)
    .await;
    match give {
        Ok(r) if r.rows_affected() == 1 => Some(source_deleted),
        Ok(r) => {
            tracing::warn!(
                player_id,
                item_id,
                occupant_item_id = occupant.item_id,
                rows_affected = r.rows_affected(),
                expected = 1,
                "MoveInventoryItem: merge give to occupant matched no row"
            );
            None
        }
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                occupant_item_id = occupant.item_id,
                "MoveInventoryItem: merge give failed: {e}"
            );
            None
        }
    }
}

/// Swap the whole source stack with the occupant, which takes the source's
/// old slot.
pub(super) async fn swap(
    tx: &mut MoveTx,
    player_id: i32,
    item_id: i32,
    source: &InventoryInstanceRow,
    occupant: &Occupant,
    target_container_id: i32,
    target_slot_id: i32,
) -> Option<()> {
    // Three-step swap to keep each statement boundary collision-free
    // against the sgw_inventory_unique_slot UNIQUE INDEX on
    // (character_id, container_id, slot_id):
    //   1. Park source at slot_id = -1 in its current container.
    //   2. Move occupant into source's original slot (now vacated).
    //   3. Move source from the sentinel slot into the target.
    //
    // A two-step swap (occupant→source's-slot, source→target) would have
    // both rows colliding on (source.container_id, source.slot_id) at the
    // end of statement 1. The sentinel slot=-1 is safe because:
    //  - bag_max_slots() never reserves negative slots, so grant/purchase
    //    paths cannot land there.
    //  - The (player_id, 0) advisory lock above serializes against other
    //    moves on this player, so no concurrent move can also be parking
    //    a different row at -1 in the same container for the same player.
    const SWAP_SENTINEL_SLOT: i32 = -1;

    let park_source = sqlx::query(
        "UPDATE sgw_inventory SET slot_id = $1 \
         WHERE character_id = $2 AND item_id = $3",
    )
    .bind(SWAP_SENTINEL_SLOT)
    .bind(player_id)
    .bind(item_id)
    .execute(&mut **tx)
    .await;
    match park_source {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(_) => {
            tracing::warn!(
                player_id,
                item_id,
                "MoveInventoryItem: park-source matched 0 rows"
            );
            return None;
        }
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                "MoveInventoryItem: park-source failed: {e}"
            );
            return None;
        }
    }

    let move_occupied = sqlx::query(
        "UPDATE sgw_inventory SET container_id = $1, slot_id = $2 \
         WHERE character_id = $3 AND item_id = $4",
    )
    .bind(source.container_id)
    .bind(source.slot_id)
    .bind(player_id)
    .bind(occupant.item_id)
    .execute(&mut **tx)
    .await;

    match move_occupied {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(_) => {
            tracing::warn!(
                player_id,
                item_id,
                occupied_item_id = occupant.item_id,
                "MoveInventoryItem: swap-occupied matched 0 rows"
            );
            return None;
        }
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                "MoveInventoryItem: swap-occupied failed: {e}"
            );
            return None;
        }
    }

    let move_source = sqlx::query(
        "UPDATE sgw_inventory SET container_id = $1, slot_id = $2 \
         WHERE character_id = $3 AND item_id = $4",
    )
    .bind(target_container_id)
    .bind(target_slot_id)
    .bind(player_id)
    .bind(item_id)
    .execute(&mut **tx)
    .await;

    match move_source {
        Ok(r) if r.rows_affected() == 1 => Some(()),
        Ok(_) => {
            tracing::warn!(
                player_id,
                item_id,
                "MoveInventoryItem: swap-source matched 0 rows"
            );
            None
        }
        Err(e) => {
            tracing::error!(player_id, item_id, "MoveInventoryItem: swap failed: {e}");
            None
        }
    }
}

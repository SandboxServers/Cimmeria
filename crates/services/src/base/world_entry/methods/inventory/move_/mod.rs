use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::super::super::super::resources::{bag_max_slots, bag_min_slot};
use super::super::super::super::ConnectedClientState;
use super::super::player_load::core::EQUIPMENT_CONTAINERS;
use super::super::vendor::helpers::sync_bandolier_after_inventory_change_with_options;
use super::appearance::refresh_player_appearance;
use super::core::send_full_inventory_update;
use super::grant::item_allows_container;
use crate::cell::messages::BaseToCellMsg;

#[derive(sqlx::FromRow)]
struct InventoryInstanceRow {
    type_id: i32,
    stack_size: i32,
    container_id: i32,
    slot_id: i32,
    bound: bool,
    durability: i32,
    charges: i32,
}

/// Move an inventory item between containers/slots, optionally swapping with occupant.
pub async fn handle_move_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_container_id: i32,
    target_slot_id: i32,
    quantity: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "MoveInventoryItem: no DB pool");
            return;
        }
    };

    let max_slots = bag_max_slots(target_container_id);
    let min_slot = bag_min_slot(target_container_id);
    // Reject out-of-range slot targets. The wire decoder is responsible for
    // translating client-side 1-indexed slot IDs into the 0-indexed values
    // this handler operates on, so a `target_slot_id < min_slot` here means
    // the client genuinely asked for a slot below the container's allowed
    // range (forged packet, off-by-one bug elsewhere).
    //
    // Quantity validation is deferred to AFTER the source row is read.
    // The SGW client's drag-to-equip / drag-to-bag UI sends
    // `quantity = -1` to mean "move the whole stack" (legacy convention
    // from `SGWPlayer.py:moveItem`). A naive `<= 0` reject here breaks
    // every drag-to-bandolier interaction. Treat `<= 0` as the
    // whole-stack sentinel, resolved against `source.stack_size`
    // once we've read the row inside the tx.
    if target_container_id <= 0 || target_slot_id < min_slot || target_slot_id >= max_slots {
        tracing::warn!(
            player_id,
            item_id,
            target_container_id,
            target_slot_id,
            quantity,
            min_slot,
            max_slots,
            "MoveInventoryItem: invalid target slot"
        );
        return;
    }

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(player_id, item_id, "MoveInventoryItem: begin failed: {e}");
            return;
        }
    };

    // Per-player advisory lock serializes ALL inventory moves for this player.
    //
    // A per-(player, container) lock is not enough: opposite-direction swaps
    // (A→B and B→A running concurrently) each lock their own source row first,
    // then deadlock when each tries to FOR-UPDATE the other's target occupant.
    // Taking a single per-player lock before any row locks eliminates that
    // ordering problem outright. Moves are rare enough that the contention
    // cost is negligible compared to the deadlock risk.
    //
    // Sentinel arg `0` distinguishes the "all-containers" move lock from the
    // per-container slot-reservation locks taken by
    // `reserve_free_inventory_slots(player_id, container_id)`.
    if let Err(e) = sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(player_id)
        .execute(&mut *tx)
        .await
    {
        if let Err(e) = tx.rollback().await {
            tracing::error!("DB rollback failed: {e}");
        }
        tracing::error!(
            player_id,
            item_id,
            target_container_id,
            "MoveInventoryItem: advisory lock failed: {e}"
        );
        return;
    }

    // Also take the per-container lock for the target so concurrent
    // grants/purchases that call `reserve_free_inventory_slots(player_id,
    // target_container)` block until the move commits. Without this, a grant
    // can read target-slot occupancy, see the slot free, INSERT into it, and
    // commit before this move's UPDATE relocates the source row — the unique
    // index on (character_id, container_id, slot_id) would then surface as a
    // user-visible error on a legitimate move. Source-container lock is taken
    // below once we've read the source row.
    if let Err(e) = sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(player_id)
        .bind(target_container_id)
        .execute(&mut *tx)
        .await
    {
        if let Err(e) = tx.rollback().await {
            tracing::error!("DB rollback failed: {e}");
        }
        tracing::error!(
            player_id,
            item_id,
            target_container_id,
            "MoveInventoryItem: target container lock failed: {e}"
        );
        return;
    }

    // The id we report on `InventoryItemMoveApplied`. Defaults to the source
    // row's id (still valid for full moves and swaps, where the same row's
    // container/slot is updated in place). The split branch overwrites this
    // with the freshly INSERTed row's id, since for a split the conceptually
    // "moved" instance is the new row in the target slot, not the decremented
    // source stack.
    let mut applied_item_id = item_id;

    // Read source row inside the tx with FOR UPDATE so concurrent moves observe
    // a consistent snapshot. Without this, the swap path could lose updates.
    let source = match sqlx::query_as::<_, InventoryInstanceRow>(
        "SELECT type_id, stack_size, container_id, slot_id, bound, durability, charges \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 LIMIT 1 FOR UPDATE",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::warn!(
                player_id,
                item_id,
                "MoveInventoryItem: source item not found"
            );
            return;
        }
        Err(e) => {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::error!(
                player_id,
                item_id,
                "MoveInventoryItem: source query failed: {e}"
            );
            return;
        }
    };

    // Resolve the whole-stack sentinel (client sends `quantity = -1`
    // for drag-to-equip / drag-to-bag — see the deferred-validation
    // note in the entry block above). Any non-positive value is
    // treated as "move the whole stack."
    let quantity = if quantity <= 0 {
        source.stack_size
    } else {
        quantity
    };

    if quantity > source.stack_size {
        if let Err(e) = tx.rollback().await {
            tracing::error!("DB rollback failed: {e}");
        }
        tracing::warn!(
            player_id,
            item_id,
            quantity,
            stack_size = source.stack_size,
            "MoveInventoryItem: requested quantity exceeds stack — rejecting"
        );
        return;
    }

    if source.container_id == target_container_id && source.slot_id == target_slot_id {
        if let Err(e) = tx.rollback().await {
            tracing::error!("DB rollback failed: {e}");
        }
        return;
    }

    // Source-container lock matches the target-container lock taken above —
    // moves where source ≠ target also need to serialize against grants into
    // the source bag (the swap path moves the displaced occupant into the
    // source's old slot and would race the same way).
    if source.container_id != target_container_id {
        if let Err(e) = sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
            .bind(player_id)
            .bind(source.container_id)
            .execute(&mut *tx)
            .await
        {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::error!(
                player_id,
                item_id,
                source_container_id = source.container_id,
                "MoveInventoryItem: source container lock failed: {e}"
            );
            return;
        }
    }

    if !item_allows_container(pool, source.type_id, target_container_id).await {
        if let Err(e) = tx.rollback().await {
            tracing::error!("DB rollback failed: {e}");
        }
        tracing::warn!(
            player_id,
            item_id,
            type_id = source.type_id,
            target_container_id,
            "MoveInventoryItem: item cannot be moved into target container"
        );
        return;
    }

    // Fetch both `item_id` and `type_id` for the occupant in one FOR-UPDATE
    // query; the row is locked here for the rest of the tx, so the swap arm
    // below can rely on the cached `type_id` instead of re-locking the same
    // row just to read its type.
    let occupied: Option<(i32, i32)> = match sqlx::query_as::<_, (i32, i32)>(
        "SELECT item_id, type_id FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 AND slot_id = $3 AND item_id <> $4 LIMIT 1 FOR UPDATE",
    )
    .bind(player_id)
    .bind(target_container_id)
    .bind(target_slot_id)
    .bind(item_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(result) => result,
        Err(e) => {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::error!(player_id, target_container_id, target_slot_id, "MoveInventoryItem: occupied slot query failed: {e}");
            return;
        }
    };

    if quantity < source.stack_size {
        if occupied.is_some() {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::warn!(
                player_id,
                item_id,
                target_container_id,
                target_slot_id,
                "MoveInventoryItem: cannot split onto occupied slot"
            );
            return;
        }

        let update = sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
             WHERE character_id = $2 AND item_id = $3 AND stack_size > $1",
        )
        .bind(quantity)
        .bind(player_id)
        .bind(item_id)
        .execute(&mut *tx)
        .await;

        let update_rows = match update {
            Ok(r) => r.rows_affected(),
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(
                    player_id,
                    item_id,
                    "MoveInventoryItem: split decrement failed: {e}"
                );
                return;
            }
        };
        if update_rows != 1 {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::warn!(
                player_id,
                item_id,
                quantity,
                "MoveInventoryItem: split decrement matched 0 rows (concurrent modification?)"
            );
            return;
        }

        // RETURNING the new row's id so `applied_item_id` can describe the
        // freshly-created instance rather than the source stack we just
        // decremented — that's what `InventoryItemMoveApplied` consumers
        // expect when they read "this is the moved item."
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
        .fetch_optional(&mut *tx)
        .await;

        match inserted {
            Ok(Some((new_id,))) => {
                applied_item_id = new_id;
                if let Err(e) = tx.commit().await {
                    tracing::error!(
                        player_id,
                        item_id,
                        "MoveInventoryItem: split commit failed: {e}"
                    );
                    return;
                }
            }
            Ok(None) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::warn!(
                    player_id,
                    item_id,
                    "MoveInventoryItem: split insert returned no row"
                );
                return;
            }
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(player_id, item_id, "MoveInventoryItem: split failed: {e}");
                return;
            }
        }
    } else if let Some((occupied_item_id, occupied_item_type)) = occupied {
        if !item_allows_container(pool, occupied_item_type, source.container_id).await {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::warn!(
                player_id,
                item_id,
                occupied_item_id,
                occupied_item_type,
                source_container_id = source.container_id,
                "MoveInventoryItem: occupied item cannot be swapped into source container"
            );
            return;
        }

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
        .execute(&mut *tx)
        .await;
        match park_source {
            Ok(r) if r.rows_affected() == 1 => {}
            Ok(_) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::warn!(
                    player_id,
                    item_id,
                    "MoveInventoryItem: park-source matched 0 rows"
                );
                return;
            }
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(
                    player_id,
                    item_id,
                    "MoveInventoryItem: park-source failed: {e}"
                );
                return;
            }
        }

        let move_occupied = sqlx::query(
            "UPDATE sgw_inventory SET container_id = $1, slot_id = $2 \
             WHERE character_id = $3 AND item_id = $4",
        )
        .bind(source.container_id)
        .bind(source.slot_id)
        .bind(player_id)
        .bind(occupied_item_id)
        .execute(&mut *tx)
        .await;

        let move_occupied_rows = match move_occupied {
            Ok(r) => r.rows_affected(),
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(
                    player_id,
                    item_id,
                    "MoveInventoryItem: swap-occupied failed: {e}"
                );
                return;
            }
        };
        if move_occupied_rows != 1 {
            if let Err(e) = tx.rollback().await {
                tracing::error!("DB rollback failed: {e}");
            }
            tracing::warn!(
                player_id,
                item_id,
                occupied_item_id,
                "MoveInventoryItem: swap-occupied matched 0 rows"
            );
            return;
        }

        let move_source = sqlx::query(
            "UPDATE sgw_inventory SET container_id = $1, slot_id = $2 \
             WHERE character_id = $3 AND item_id = $4",
        )
        .bind(target_container_id)
        .bind(target_slot_id)
        .bind(player_id)
        .bind(item_id)
        .execute(&mut *tx)
        .await;

        match move_source {
            Ok(r) if r.rows_affected() == 1 => {
                if let Err(e) = tx.commit().await {
                    tracing::error!(
                        player_id,
                        item_id,
                        "MoveInventoryItem: swap commit failed: {e}"
                    );
                    return;
                }
            }
            Ok(_) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::warn!(
                    player_id,
                    item_id,
                    "MoveInventoryItem: swap-source matched 0 rows"
                );
                return;
            }
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(player_id, item_id, "MoveInventoryItem: swap failed: {e}");
                return;
            }
        }
    } else {
        let result = sqlx::query(
            "UPDATE sgw_inventory SET container_id = $1, slot_id = $2 \
             WHERE character_id = $3 AND item_id = $4",
        )
        .bind(target_container_id)
        .bind(target_slot_id)
        .bind(player_id)
        .bind(item_id)
        .execute(&mut *tx)
        .await;

        match result {
            Ok(r) if r.rows_affected() == 1 => {
                if let Err(e) = tx.commit().await {
                    tracing::error!(
                        player_id,
                        item_id,
                        "MoveInventoryItem: simple commit failed: {e}"
                    );
                    return;
                }
            }
            Ok(_) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::warn!(player_id, item_id, "MoveInventoryItem: no rows updated");
                return;
            }
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("DB rollback failed: {e}");
                }
                tracing::error!(player_id, item_id, "MoveInventoryItem: update failed: {e}");
                return;
            }
        }
    }

    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        socket,
        connected,
        entity_to_addr,
    )
    .await;

    tracing::debug!(
        entity_id,
        player_id,
        item_id,
        total_items,
        "Inventory move persisted"
    );

    if let Some(cell_tx) = cell_tx {
        if let Err(e) = cell_tx
            .send(BaseToCellMsg::InventoryItemMoveApplied {
                entity_id,
                item_id: applied_item_id,
                type_id: source.type_id,
                source_container_id: source.container_id,
                target_container_id,
                swapped_item_id: occupied.map(|(id, _)| id),
            })
            .await
        {
            tracing::warn!(entity_id, "InventoryItemMoveApplied send failed: {e}");
        }
    }

    if source.container_id == 3 || target_container_id == 3 {
        // Unequip (source=bandolier, target=elsewhere): defer the
        // base-side `refresh_player_appearance` so the cell-side
        // holster animation has time to play. The cell's
        // `SyncBandolierItems` handler fires `Item_Unequip` and
        // schedules a Phase 2 (`holster_animation_complete_at`) that
        // dispatches the eventual `RefreshAppearance` back to base
        // after `HOLSTER_ANIMATION_DURATION`. Without this defer,
        // the base yanks the weapon mesh immediately and the user
        // sees no animation — the weapon just vanishes.
        let is_unequip = source.container_id == 3 && target_container_id != 3;
        sync_bandolier_after_inventory_change_with_options(
            entity_id,
            player_id,
            db_pool,
            cell_tx,
            socket,
            connected,
            entity_to_addr,
            is_unequip,
        )
        .await;
    }

    // Equipment containers (4..=14) — armor and other slotted visuals.
    // The grant path already refreshes appearance on equipment grants;
    // the bandolier branch above handles weapons. Without this branch,
    // manually dragging armor into (or out of) a slot persists to DB but
    // the player-visible model on every client keeps the pre-move
    // components.
    //
    // Gated on `visual_component IS NOT NULL` to match the grant path's
    // shape (grant\mod.rs:425) — non-visual items (charms, ID-only
    // artifacts) can legally occupy equipment slots without contributing
    // to the appearance composite, so refreshing for them is wasted
    // wire traffic. Lookup is keyed by `source.type_id`, which is the
    // type both the equip leg (bag→slot) and the unequip leg (slot→bag)
    // are moving; for a swap, only the source item's visual matters
    // (the displaced occupant's container also changes, but that case
    // is already covered when the swap's source/target straddles
    // equipment).
    if EQUIPMENT_CONTAINERS.contains(&source.container_id)
        || EQUIPMENT_CONTAINERS.contains(&target_container_id)
    {
        let has_visual: bool = sqlx::query_scalar(
            "SELECT visual_component IS NOT NULL FROM resources.items WHERE item_id = $1",
        )
        .bind(source.type_id)
        .fetch_optional(pool.as_ref())
        .await
        .ok()
        .flatten()
        .unwrap_or(false);

        if has_visual {
            refresh_player_appearance(
                entity_id,
                player_id,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod concurrency_tests;
#[cfg(test)]
mod tests;

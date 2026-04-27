use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use super::super::super::helpers::send_to_witness;
use super::super::super::resources::bag_max_slots;
use super::super::super::ConnectedClientState;
use super::core::send_full_inventory_update;
use super::grant::item_allows_container;
use super::super::vendor::helpers::sync_bandolier_after_inventory_change;

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
    if target_container_id <= 0 || target_slot_id < 0 || target_slot_id >= max_slots || quantity <= 0 {
        tracing::warn!(
            player_id, item_id, target_container_id, target_slot_id,
            quantity, max_slots, "MoveInventoryItem: invalid target or quantity"
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
        let _ = tx.rollback().await;
        tracing::error!(player_id, item_id, target_container_id, "MoveInventoryItem: advisory lock failed: {e}");
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
        let _ = tx.rollback().await;
        tracing::error!(player_id, item_id, target_container_id, "MoveInventoryItem: target container lock failed: {e}");
        return;
    }

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
            let _ = tx.rollback().await;
            tracing::warn!(player_id, item_id, "MoveInventoryItem: source item not found");
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(player_id, item_id, "MoveInventoryItem: source query failed: {e}");
            return;
        }
    };

    if quantity > source.stack_size {
        let _ = tx.rollback().await;
        tracing::warn!(
            player_id, item_id, quantity, stack_size = source.stack_size,
            "MoveInventoryItem: requested quantity exceeds stack — rejecting"
        );
        return;
    }

    if source.container_id == target_container_id && source.slot_id == target_slot_id {
        let _ = tx.rollback().await;
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
            let _ = tx.rollback().await;
            tracing::error!(player_id, item_id, source_container_id = source.container_id, "MoveInventoryItem: source container lock failed: {e}");
            return;
        }
    }

    if !item_allows_container(pool, source.type_id, target_container_id).await {
        let _ = tx.rollback().await;
        tracing::warn!(
            player_id, item_id, type_id = source.type_id, target_container_id,
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
            let _ = tx.rollback().await;
            tracing::error!(player_id, target_container_id, target_slot_id, "MoveInventoryItem: occupied slot query failed: {e}");
            return;
        }
    };

    if quantity < source.stack_size {
        if occupied.is_some() {
            let _ = tx.rollback().await;
            tracing::warn!(
                player_id, item_id, target_container_id, target_slot_id,
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
                let _ = tx.rollback().await;
                tracing::error!(player_id, item_id, "MoveInventoryItem: split decrement failed: {e}");
                return;
            }
        };
        if update_rows != 1 {
            let _ = tx.rollback().await;
            tracing::warn!(
                player_id, item_id, quantity,
                "MoveInventoryItem: split decrement matched 0 rows (concurrent modification?)"
            );
            return;
        }

        let insert = sqlx::query(
            "INSERT INTO sgw_inventory \
             (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(player_id)
        .bind(source.type_id)
        .bind(quantity)
        .bind(target_slot_id)
        .bind(target_container_id)
        .bind(source.bound)
        .bind(source.durability)
        .bind(source.charges)
        .execute(&mut *tx)
        .await;

        match insert {
            Ok(r) if r.rows_affected() == 1 => {
                if let Err(e) = tx.commit().await {
                    tracing::error!(player_id, item_id, "MoveInventoryItem: split commit failed: {e}");
                    return;
                }
            }
            Ok(_) => {
                let _ = tx.rollback().await;
                tracing::warn!(player_id, item_id, "MoveInventoryItem: split insert affected 0 rows");
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(player_id, item_id, "MoveInventoryItem: split failed: {e}");
                return;
            }
        }
    } else if let Some((occupied_item_id, occupied_item_type)) = occupied {
        if !item_allows_container(pool, occupied_item_type, source.container_id).await {
            let _ = tx.rollback().await;
            tracing::warn!(
                player_id, item_id, occupied_item_id, occupied_item_type,
                source_container_id = source.container_id,
                "MoveInventoryItem: occupied item cannot be swapped into source container"
            );
            return;
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
                let _ = tx.rollback().await;
                tracing::error!(player_id, item_id, "MoveInventoryItem: swap-occupied failed: {e}");
                return;
            }
        };
        if move_occupied_rows != 1 {
            let _ = tx.rollback().await;
            tracing::warn!(
                player_id, item_id, occupied_item_id,
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
                    tracing::error!(player_id, item_id, "MoveInventoryItem: swap commit failed: {e}");
                    return;
                }
            }
            Ok(_) => {
                let _ = tx.rollback().await;
                tracing::warn!(player_id, item_id, "MoveInventoryItem: swap-source matched 0 rows");
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
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
                    tracing::error!(player_id, item_id, "MoveInventoryItem: simple commit failed: {e}");
                    return;
                }
            }
            Ok(_) => {
                let _ = tx.rollback().await;
                tracing::warn!(player_id, item_id, "MoveInventoryItem: no rows updated");
                return;
            }
            Err(e) => {
                let _ = tx.rollback().await;
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
        entity_id, player_id, item_id, total_items,
        "Inventory move persisted"
    );

    if let Some(cell_tx) = cell_tx {
        let _ = cell_tx
            .send(BaseToCellMsg::InventoryItemMoveApplied {
                entity_id,
                item_id,
                source_container_id: source.container_id,
                target_container_id,
                swapped_item_id: occupied.map(|(id, _)| id),
            })
            .await;
    }

    if source.container_id == 3 || target_container_id == 3 {
        sync_bandolier_after_inventory_change(
            entity_id,
            player_id,
            db_pool,
            cell_tx,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
    }
}
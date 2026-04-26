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

    let source = sqlx::query_as::<_, InventoryInstanceRow>(
        "SELECT type_id, stack_size, container_id, slot_id, bound, durability, charges \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 LIMIT 1",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    .ok()
    .flatten();

    let Some(source) = source else {
        tracing::warn!(player_id, item_id, "MoveInventoryItem: source item not found");
        return;
    };

    let max_slots = bag_max_slots(target_container_id);
    if target_container_id <= 0 || target_slot_id < 0 || target_slot_id >= max_slots || quantity <= 0 {
        tracing::warn!(
            player_id, item_id, target_container_id, target_slot_id,
            quantity, max_slots, "MoveInventoryItem: invalid target or quantity"
        );
        return;
    }

    if source.container_id == target_container_id && source.slot_id == target_slot_id {
        return;
    }

    if !item_allows_container(pool, source.type_id, target_container_id).await {
        tracing::warn!(
            player_id, item_id, type_id = source.type_id, target_container_id,
            "MoveInventoryItem: item cannot be moved into target container"
        );
        return;
    }

    let occupied: Option<i32> = sqlx::query_scalar(
        "SELECT item_id FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 AND slot_id = $3 AND item_id <> $4 LIMIT 1",
    )
    .bind(player_id)
    .bind(target_container_id)
    .bind(target_slot_id)
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    .ok()
    .flatten();

    if quantity < source.stack_size {
        if occupied.is_some() {
            tracing::warn!(
                player_id, item_id, target_container_id, target_slot_id,
                "MoveInventoryItem: cannot split onto occupied slot"
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

        let update = sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
             WHERE character_id = $2 AND item_id = $3 AND stack_size > $1",
        )
        .bind(quantity)
        .bind(player_id)
        .bind(item_id)
        .execute(&mut *tx)
        .await;

        let insert = if update.as_ref().map(|r| r.rows_affected()).unwrap_or(0) == 1 {
            sqlx::query(
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
            .await
        } else {
            update
        };

        match insert {
            Ok(_) => {
                let _ = tx.commit().await;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(player_id, item_id, "MoveInventoryItem: split failed: {e}");
                return;
            }
        }
    } else if let Some(occupied_item_id) = occupied {
        let occupied_item_type: Option<i32> = sqlx::query_scalar(
            "SELECT type_id FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 LIMIT 1",
        )
        .bind(player_id)
        .bind(occupied_item_id)
        .fetch_optional(pool.as_ref())
        .await
        .ok()
        .flatten();

        let Some(occupied_item_type) = occupied_item_type else {
            tracing::warn!(
                player_id, item_id, occupied_item_id,
                "MoveInventoryItem: occupied item disappeared before swap"
            );
            return;
        };

        if !item_allows_container(pool, occupied_item_type, source.container_id).await {
            tracing::warn!(
                player_id, item_id, occupied_item_id, occupied_item_type,
                source_container_id = source.container_id,
                "MoveInventoryItem: occupied item cannot be swapped into source container"
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

        let move_source = if move_occupied.as_ref().map(|r| r.rows_affected()).unwrap_or(0) == 1 {
            sqlx::query(
                "UPDATE sgw_inventory SET container_id = $1, slot_id = $2 \
                 WHERE character_id = $3 AND item_id = $4",
            )
            .bind(target_container_id)
            .bind(target_slot_id)
            .bind(player_id)
            .bind(item_id)
            .execute(&mut *tx)
            .await
        } else {
            move_occupied
        };

        match move_source {
            Ok(_) => {
                let _ = tx.commit().await;
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
        .execute(pool.as_ref())
        .await;

        match result {
            Ok(r) if r.rows_affected() == 1 => {}
            Ok(_) => {
                tracing::warn!(player_id, item_id, "MoveInventoryItem: no rows updated");
                return;
            }
            Err(e) => {
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
                swapped_item_id: occupied,
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
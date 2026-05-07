//! `handle_remove_inventory_item` — remove a specific inventory instance
//! by `item_id` (instance id from the wire), with optional partial
//! decrement when `quantity < stack_size`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::super::super::super::super::ConnectedClientState;
use super::super::super::vendor::helpers::sync_bandolier_after_inventory_change;
use super::{send_full_inventory_update, send_on_remove_item, InventoryInstanceRow};
use crate::base::outbox::{self, CellOutboxPayload};
use crate::cell::messages::BaseToCellMsg;

/// Remove an inventory item from player inventory and sync client.
#[allow(clippy::too_many_arguments)]
pub async fn handle_remove_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
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
            tracing::debug!(player_id, item_id, "RemoveInventoryItem: no DB pool");
            return;
        }
    };

    if quantity <= 0 {
        tracing::warn!(
            player_id,
            item_id,
            quantity,
            "RemoveInventoryItem: invalid quantity"
        );
        return;
    }

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                "RemoveInventoryItem: begin tx failed: {e}"
            );
            return;
        }
    };

    let source = match sqlx::query_as::<_, InventoryInstanceRow>(
        "SELECT stack_size, container_id \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 LIMIT 1 FOR UPDATE",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(opt) => opt,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                player_id,
                item_id,
                "RemoveInventoryItem: source query failed: {e}"
            );
            return;
        }
    };

    let Some(source) = source else {
        let _ = tx.rollback().await;
        tracing::warn!(
            player_id,
            item_id,
            "RemoveInventoryItem: source item not found"
        );
        return;
    };

    let removed_all = quantity >= source.stack_size;
    let result = if removed_all {
        sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1 AND item_id = $2")
            .bind(player_id)
            .bind(item_id)
            .execute(&mut *tx)
            .await
    } else {
        sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
             WHERE character_id = $2 AND item_id = $3 AND stack_size > $1",
        )
        .bind(quantity)
        .bind(player_id)
        .bind(item_id)
        .execute(&mut *tx)
        .await
    };

    match result {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(_) => {
            let _ = tx.rollback().await;
            tracing::warn!(player_id, item_id, "RemoveInventoryItem: no rows changed");
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                player_id,
                item_id,
                "RemoveInventoryItem: update failed: {e}"
            );
            return;
        }
    }

    // Enqueue the cell-notification BEFORE commit so the outbox row and the
    // inventory mutation become visible atomically. Only fired on full removal
    // — partial decrement doesn't change which item-instances exist on the
    // cell side. If outbox INSERT fails we abort the remove rather than leave
    // the inventory mutated without a durable notification path.
    let outbox_payload_id = if removed_all {
        let payload = CellOutboxPayload::InventoryItemRemoved {
            item_id,
            source_container_id: source.container_id,
        };
        match outbox::enqueue_in_tx(&mut tx, entity_id, &payload).await {
            Ok(id) => Some((id, payload)),
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    player_id,
                    item_id,
                    "RemoveInventoryItem: outbox enqueue failed, aborting: {e}"
                );
                return;
            }
        }
    } else {
        None
    };

    if let Err(e) = tx.commit().await {
        tracing::error!(
            player_id,
            item_id,
            "RemoveInventoryItem: commit failed: {e}"
        );
        return;
    }

    if removed_all {
        send_on_remove_item(entity_id, item_id, socket, connected, entity_to_addr).await;
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
        quantity,
        total_items,
        "Inventory remove persisted"
    );

    if let (Some((outbox_id, payload)), Some(tx)) = (outbox_payload_id, cell_tx) {
        outbox::try_dispatch_now(pool.as_ref(), tx, outbox_id, entity_id, payload).await;
    }

    if source.container_id == 3 {
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

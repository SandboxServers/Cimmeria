//! `handle_remove_inventory_item_by_type` — find and consume the player's
//! first instance of a given design id. Used by `Action::RemoveItem` in the
//! cell content executor (chains know item design ids, not instance ids).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::super::super::super::ConnectedClientState;
use super::super::super::vendor::helpers::sync_bandolier_after_inventory_change;
use super::{send_full_inventory_update, send_on_remove_item, InventoryInstanceWithIdRow};
use crate::base::outbox::{self, CellOutboxPayload};
use crate::cell::messages::BaseToCellMsg;

/// Resolve a player's first inventory instance with the given design
/// `type_id` and remove `count` from it (delete the row when the stack is
/// fully drained). Used by `Action::RemoveItem` in the cell content
/// executor — chains know item design ids, not instance ids.
///
/// Sends the same client wire-update sequence as
/// [`super::handle_remove_inventory_item`]: `onRemoveItem` (when the row is
/// deleted) → full inventory refresh → `BaseToCellMsg::InventoryItemRemoved`
/// (when the row is deleted). Bandolier sync runs when the source row was
/// in container 3.
#[tracing::instrument(
    name = "inventory.remove_by_type",
    level = "info",
    skip_all,
    fields(entity_id, player_id, type_id, count)
)]
pub async fn handle_remove_inventory_item_by_type(
    entity_id: u32,
    player_id: i32,
    type_id: i32,
    count: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, type_id, "RemoveInventoryItemByType: no DB pool");
            return;
        }
    };

    if count <= 0 {
        tracing::warn!(
            player_id,
            type_id,
            count,
            "RemoveInventoryItemByType: invalid count"
        );
        return;
    }

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                player_id,
                type_id,
                "RemoveInventoryItemByType: begin tx failed: {e}"
            );
            return;
        }
    };

    // Resolve the first matching instance for this design id. FOR UPDATE
    // because we're about to mutate; LIMIT 1 because we only consume from
    // a single stack — multi-stack draining isn't the chain semantic
    // (mission removes are always 1 from a specific stack).
    //
    // ORDER BY (container_id, slot_id) is load-bearing — without it, sqlx
    // returns whichever row PG felt like; for a player with stacks of the
    // same design in both the main bag (container 1) and the bandolier
    // (container 3), an unordered LIMIT 1 might consume the equipped
    // stack instead of the main-bag one. The ascending order biases
    // toward main bag (container 1) which is the desired default for
    // chain-driven consumes (vials, mission objects) — bandolier stacks
    // are touched by explicit equip/unequip flows, not by chains.
    //
    // The SELECT also pulls `item_id` so we don't need a second roundtrip
    // to look it up before sending the targeted onRemoveItem packet on
    // full removal.
    let source = match sqlx::query_as::<_, InventoryInstanceWithIdRow>(
        "SELECT item_id, stack_size, container_id, slot_id \
         FROM sgw_inventory WHERE character_id = $1 AND type_id = $2 \
         ORDER BY container_id, slot_id LIMIT 1 FOR UPDATE",
    )
    .bind(player_id)
    .bind(type_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(opt) => opt,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                player_id,
                type_id,
                "RemoveInventoryItemByType: source query failed: {e}"
            );
            return;
        }
    };

    let Some(source) = source else {
        let _ = tx.rollback().await;
        tracing::warn!(
            player_id,
            type_id,
            "RemoveInventoryItemByType: no instance of this design id owned by character"
        );
        return;
    };

    let removed_item_id = source.item_id;

    if count > source.stack_size {
        // Visible warning — caller asked for more than this stack holds.
        // We still proceed (treating it as "remove the whole stack"),
        // matching the existing handle_remove_inventory_item semantic
        // for `quantity >= stack_size`. If a future caller actually
        // needs multi-stack draining, that's a new RPC variant, not a
        // silent extension of this one.
        tracing::warn!(
            player_id,
            type_id,
            requested = count,
            available = source.stack_size,
            "RemoveInventoryItemByType: requested count exceeds stack size; removing whole stack"
        );
    }

    let removed_all = count >= source.stack_size;
    let result = if removed_all {
        sqlx::query(
            "DELETE FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = $2 AND slot_id = $3",
        )
        .bind(player_id)
        .bind(source.container_id)
        .bind(source.slot_id)
        .execute(&mut *tx)
        .await
    } else {
        sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
             WHERE character_id = $2 AND container_id = $3 AND slot_id = $4 AND stack_size > $1",
        )
        .bind(count)
        .bind(player_id)
        .bind(source.container_id)
        .bind(source.slot_id)
        .execute(&mut *tx)
        .await
    };

    match result {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(r) => {
            let rows = r.rows_affected();
            let _ = tx.rollback().await;
            // include rows_affected + expected so a single
            // ops query (rows_affected != expected) surfaces every
            // divergence in one place.
            tracing::warn!(
                player_id,
                type_id,
                rows_affected = rows,
                expected = 1,
                "RemoveInventoryItemByType: no rows changed -- item missing or stack underflow"
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                player_id,
                type_id,
                "RemoveInventoryItemByType: update failed: {e}"
            );
            return;
        }
    }

    let outbox_payload_id = if removed_all {
        let payload = CellOutboxPayload::InventoryItemRemoved {
            item_id: removed_item_id,
            source_container_id: source.container_id,
        };
        match outbox::enqueue_in_tx(&mut tx, entity_id, &payload).await {
            Ok(id) => Some((id, payload)),
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    player_id,
                    type_id,
                    "RemoveInventoryItemByType: outbox enqueue failed, aborting: {e}"
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
            type_id,
            "RemoveInventoryItemByType: commit failed: {e}"
        );
        return;
    }

    if removed_all {
        send_on_remove_item(
            entity_id,
            removed_item_id,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;

    tracing::info!(
        entity_id,
        player_id,
        type_id,
        count,
        removed_all,
        total_items,
        "RemoveInventoryItemByType: persisted"
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
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
}

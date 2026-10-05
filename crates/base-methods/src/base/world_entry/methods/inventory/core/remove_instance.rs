//! `handle_remove_inventory_item` — remove a specific inventory instance
//! by `item_id` (instance id from the wire), with optional partial
//! decrement when `quantity < stack_size`.
//!
//! [`remove_instance`] is the body, shared with the native consumable's
//! consume ([`super::consume_for_use`]), which also needs to know whether
//! the unit was really taken and to hold the row to one design id.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::super::super::super::gm_feedback::send_gm_feedback_to_client;
use super::super::super::super::super::ConnectedClientState;
use super::super::super::vendor::helpers::sync_bandolier_after_inventory_change;
use super::super::move_::player_accessible;
use super::access::{refuse_inaccessible, AccessOp};
use super::{send_full_inventory_update, send_on_remove_item, InventoryInstanceRow};
use crate::base::outbox::{self, CellOutboxPayload};
use crate::cell::messages::BaseToCellMsg;
use cimmeria_wire::cell::vault::VaultAccess;

/// Remove an inventory item from player inventory and sync client.
#[tracing::instrument(
    name = "inventory.remove_item",
    level = "info",
    skip_all,
    fields(entity_id, player_id, item_id, quantity)
)]
pub async fn handle_remove_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    quantity: i32,
    notify_gm: bool,
    vault: VaultAccess,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let _ = remove_instance(
        RemoveInstance {
            entity_id,
            player_id,
            item_id,
            quantity,
            notify_gm,
            vault,
            expected_type_id: None,
            op: AccessOp::Remove,
        },
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// One instance removal, as [`remove_instance`] takes it.
pub(super) struct RemoveInstance {
    pub(super) entity_id: u32,
    pub(super) player_id: i32,
    /// The inventory instance (`sgw_inventory.item_id`).
    pub(super) item_id: i32,
    pub(super) quantity: i32,
    pub(super) notify_gm: bool,
    pub(super) vault: VaultAccess,
    /// Remove only a row of this design id; any other type is "not found".
    pub(super) expected_type_id: Option<i32>,
    /// The operation an inaccessible container refuses, for its feedback.
    pub(super) op: AccessOp,
}

/// The removal body. Returns `true` only when the removal committed.
pub(super) async fn remove_instance(
    req: RemoveInstance,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool {
    let RemoveInstance {
        entity_id,
        player_id,
        item_id,
        quantity,
        notify_gm,
        vault,
        expected_type_id,
        op,
    } = req;
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(
                player_id,
                player_name = known_names::player_name(player_id),
                item_id,
                item_name = cimmeria_names::owned::item(expected_type_id),
                "RemoveInventoryItem: no DB pool"
            );
            return false;
        }
    };

    if quantity <= 0 {
        tracing::warn!(
            player_id,
            player_name = known_names::player_name(player_id),
            item_id,
            item_name = cimmeria_names::owned::item(expected_type_id),
            quantity,
            "RemoveInventoryItem: invalid quantity"
        );
        return false;
    }

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                player_id,
                player_name = known_names::player_name(player_id),
                item_id,
                item_name = cimmeria_names::owned::item(expected_type_id),
                "RemoveInventoryItem: begin tx failed: {e}"
            );
            return false;
        }
    };

    let source = match sqlx::query_as::<_, InventoryInstanceRow>(
        "SELECT stack_size, container_id, type_id \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 \
           AND ($3::int IS NULL OR type_id = $3) \
         LIMIT 1 FOR UPDATE",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(expected_type_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(opt) => opt,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                player_id,
                player_name = known_names::player_name(player_id),
                item_id,
                item_name = cimmeria_names::owned::item(expected_type_id),
                "RemoveInventoryItem: source query failed: {e}"
            );
            return false;
        }
    };

    let Some(source) = source else {
        let _ = tx.rollback().await;
        tracing::warn!(
            player_id,
            player_name = known_names::player_name(player_id),
            item_id, // nt:id-only instance id, no such row to type
            expected_item_type_id = expected_type_id,
            expected_item_name = cimmeria_names::owned::item(expected_type_id),
            "RemoveInventoryItem: source item not found (or not of the expected type)"
        );
        return false;
    };

    // Only an item the player can reach may be removed (BV-03): not one in
    // buyback, and one in the vault only with a vault session open. The GM
    // `gmRemoveItem` path is held to the same rule for the GM's own items.
    if !player_accessible(source.container_id, &vault) {
        let _ = tx.rollback().await;
        refuse_inaccessible(
            op,
            entity_id,
            player_id,
            item_id,
            source.container_id,
            &vault,
            pool,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return false;
    }

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
        Ok(r) => {
            let rows = r.rows_affected();
            let _ = tx.rollback().await;
            // include rows_affected + expected so a single
            // ops query (rows_affected != expected) surfaces every
            // divergence in one place.
            tracing::warn!(
                player_id,
                player_name = known_names::player_name(player_id),
                item_id,
                item_name = cimmeria_names::owned::item(source.type_id),
                rows_affected = rows,
                expected = 1,
                "RemoveInventoryItem: no rows changed -- item missing or stack underflow"
            );
            return false;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                player_id,
                player_name = known_names::player_name(player_id),
                item_id,
                item_name = cimmeria_names::owned::item(source.type_id),
                "RemoveInventoryItem: update failed: {e}"
            );
            return false;
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
                    player_name = known_names::player_name(player_id),
                    item_id,
                    item_name = cimmeria_names::owned::item(source.type_id),
                    "RemoveInventoryItem: outbox enqueue failed, aborting: {e}"
                );
                return false;
            }
        }
    } else {
        None
    };

    if let Err(e) = tx.commit().await {
        tracing::error!(
            player_id,
            player_name = known_names::player_name(player_id),
            item_id,
            item_name = cimmeria_names::owned::item(source.type_id),
            "RemoveInventoryItem: commit failed: {e}"
        );
        return false;
    }

    if removed_all {
        send_on_remove_item(entity_id, item_id, transport, connected, entity_to_addr).await;
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

    let player_label = known_names::player_name(player_id);
    tracing::debug!(
        entity_id,
        entity_name = player_label,
        player_id,
        player_name = player_label,
        item_id,
        item_name = cimmeria_names::owned::item(source.type_id),
        quantity,
        total_items,
        "Inventory remove persisted"
    );

    // Definitive GM feedback — only for GM-sourced removes (`gmRemoveItem`).
    // Player drops / content-chain removes leave `notify_gm` false. Fired
    // post-commit so it reflects an actual removal.
    if notify_gm {
        // On a full delete the row may hold fewer than the requested quantity, so
        // report the count that actually committed rather than what was asked for.
        let removed_qty = if removed_all {
            source.stack_size
        } else {
            quantity
        };
        send_gm_feedback_to_client(
            entity_id,
            &format!("gmRemoveItem: removed {removed_qty}x item {item_id}"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

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

    true
}

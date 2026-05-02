use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};
use super::super::super::super::helpers::send_to_witness;
use super::super::super::super::ConnectedClientState;
use super::super::vendor::helpers::sync_bandolier_after_inventory_change;

const INVENTORY_ITEM_SELECT: &str = r#"
SELECT inv.item_id, inv.type_id, inv.stack_size, inv.slot_id, inv.container_id,
       inv.bound, inv.durability, inv.charges,
       COALESCE((
           SELECT array_agg(array_position(enum_range(NULL::resources."EAmmoType"), ammo) - 1 ORDER BY ord)
           FROM unnest(ri.ammo_types) WITH ORDINALITY AS ammo_values(ammo, ord)
       ), ARRAY[]::integer[]) AS ammo_type_ids,
       CASE WHEN ri.default_ammo_type IS NULL THEN 0
            ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
       END AS cur_ammo_type_id
FROM sgw_inventory inv
LEFT JOIN resources.items ri ON ri.item_id = inv.type_id
WHERE inv.character_id = $1
ORDER BY inv.container_id, inv.slot_id
"#;

#[derive(sqlx::FromRow)]
struct InventoryRow {
    item_id: i32,
    type_id: i32,
    stack_size: i32,
    slot_id: i32,
    container_id: i32,
    bound: bool,
    durability: i32,
    charges: i32,
    ammo_type_ids: Vec<i32>,
    cur_ammo_type_id: i32,
}

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

/// Send full inventory update to player, refreshing all items on the client.
pub async fn send_full_inventory_update(
    entity_id: u32,
    player_id: i32,
    pool: &Arc<PgPool>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> usize {
    let all_items: Vec<InventoryRow> =
        match sqlx::query_as::<_, InventoryRow>(INVENTORY_ITEM_SELECT)
            .bind(player_id)
            .fetch_all(pool.as_ref())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(
                    entity_id,
                    player_id,
                    "send_full_inventory_update: query failed, refusing to broadcast empty inventory: {e}"
                );
                return 0;
            }
        };

    let mut args = Vec::with_capacity(4 + all_items.len() * 48);
    args.extend_from_slice(&(all_items.len() as u32).to_le_bytes());
    for row in all_items.iter() {
        let item = cimmeria_entity::inventory::InvItem {
            id: row.item_id,
            dbid: row.type_id,
            stack_size: row.stack_size,
            slot_id: row.slot_id + 1,
            container_id: row.container_id,
            is_bound: row.bound,
            durability: row.durability,
            ammo_types: row.ammo_type_ids.clone(),
            cur_ammo_type: row.cur_ammo_type_id,
            charges: row.charges,
        };
        item.serialize(&mut args);
    }

    send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(key, seq, acks, entity_id, method_idx::ON_UPDATE_ITEM, &args)
        },
    )
    .await;

    all_items.len()
}

/// Tell the player's client to drop an inventory item instance from its
/// local UI cache.
///
/// `onUpdateItem` (used by `send_full_inventory_update`) is upsert-only —
/// when a stack is fully removed, the client won't drop it just because the
/// next full-inventory packet omits it. This fires the explicit
/// `onRemoveItem(ItemIdList)` per the SGWInventoryManager interface so the
/// slot actually clears in the UI.
///
/// Call after the DB commit, before `send_full_inventory_update`.
async fn send_on_remove_item(
    entity_id: u32,
    item_id: i32,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&1u32.to_le_bytes()); // ARRAY<INT32> count
    args.extend_from_slice(&item_id.to_le_bytes());
    send_to_witness(
        socket, connected, entity_to_addr, entity_id,
        |key, seq, acks| {
            build_entity_method_packet(key, seq, acks, entity_id, method_idx::ON_REMOVE_ITEM, &args)
        },
    ).await;
}

/// Remove an inventory item from player inventory and sync client.
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
            tracing::error!(player_id, item_id, "RemoveInventoryItem: begin tx failed: {e}");
            return;
        }
    };

    let source = match sqlx::query_as::<_, InventoryInstanceRow>(
        "SELECT type_id, stack_size, container_id, slot_id, bound, durability, charges \
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
            tracing::error!(player_id, item_id, "RemoveInventoryItem: source query failed: {e}");
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

    if let Err(e) = tx.commit().await {
        tracing::error!(player_id, item_id, "RemoveInventoryItem: commit failed: {e}");
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

    if removed_all {
        if let Some(cell_tx) = cell_tx {
            let _ = cell_tx
                .send(BaseToCellMsg::InventoryItemRemoved {
                    entity_id,
                    item_id,
                    source_container_id: source.container_id,
                })
                .await;
        }
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

/// Atomically consume one charge of an inventory item, then notify the cell
/// with the design id (`type_id`) so the content engine can fire `OnItemUse`.
///
/// `item_id` here is the inventory **instance id** from the wire (per
/// `SGWInventoryManager.def`'s `useItem` arg). The cell never knows the
/// design id at use-time — base resolves it from `sgw_inventory` and sends
/// it back via `BaseToCellMsg::ItemUsed`.
///
/// Mission progression that listens for `OnItemUse` only fires after the
/// consumption tx commits — if the player doesn't own that instance, or
/// the tx fails, nothing is sent back and the chain doesn't progress.
///
/// TODO(consume-on-use): consumption is currently unconditional. The Python
/// reference decoupled "fire `item.use::<typeId>` event" from "remove from
/// inventory" — script callbacks decided per item type. That meant
/// reusable quest items (radio-style "use on target" objectives, multi-step
/// items) could fire the event many times. Once content chains start
/// driving multi-use items, split this into a "resolve type_id + fire
/// event" path and a separate "consume by design id" base RPC that chain
/// `Action::RemoveItem` can call. For Ambernol (the only currently-shipped
/// `OnItemUse` consumer) the always-consume behavior is correct.
///
/// TODO(delivery durability): the `ItemUsed` send below is best-effort. If
/// `tx.commit()` succeeds but `cell_tx` is closed (cell service restart,
/// channel saturation), the item is gone from the DB but `OnItemUse` never
/// fires — mission progression that gates on it strands the player. For a
/// single-operator deployment this is acceptable. Production-grade fix is
/// an outbox row written in the same transaction, drained by a retrier.
pub async fn handle_use_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "UseInventoryItem: no DB pool");
            return;
        }
    };

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(player_id, item_id, "UseInventoryItem: begin tx failed: {e}");
            return;
        }
    };

    let source = match sqlx::query_as::<_, InventoryInstanceRow>(
        "SELECT type_id, stack_size, container_id, slot_id, bound, durability, charges \
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
            tracing::error!(player_id, item_id, "UseInventoryItem: source query failed: {e}");
            return;
        }
    };

    let Some(source) = source else {
        let _ = tx.rollback().await;
        tracing::warn!(
            player_id, item_id,
            "UseInventoryItem: instance not found for this character — refusing to fire ItemUsed"
        );
        return;
    };

    let consumed_all = source.stack_size <= 1;
    let result = if consumed_all {
        sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1 AND item_id = $2")
            .bind(player_id)
            .bind(item_id)
            .execute(&mut *tx)
            .await
    } else {
        sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - 1 \
             WHERE character_id = $1 AND item_id = $2 AND stack_size > 1",
        )
        .bind(player_id)
        .bind(item_id)
        .execute(&mut *tx)
        .await
    };

    match result {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(_) => {
            let _ = tx.rollback().await;
            tracing::warn!(player_id, item_id, "UseInventoryItem: no rows changed");
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(player_id, item_id, "UseInventoryItem: consume failed: {e}");
            return;
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(player_id, item_id, "UseInventoryItem: commit failed: {e}");
        return;
    }

    if consumed_all {
        send_on_remove_item(entity_id, item_id, socket, connected, entity_to_addr).await;
    }

    let total_items = send_full_inventory_update(
        entity_id, player_id, pool, socket, connected, entity_to_addr,
    ).await;

    tracing::info!(
        entity_id, player_id, item_id,
        type_id = source.type_id, target_id, total_items, consumed_all,
        "UseInventoryItem: consumed, firing ItemUsed back to cell"
    );

    if let Some(cell_tx) = cell_tx {
        if let Err(e) = cell_tx.send(BaseToCellMsg::ItemUsed {
            entity_id,
            type_id: source.type_id,
            target_id,
        }).await {
            tracing::error!(
                entity_id, player_id, item_id, error = %e,
                "UseInventoryItem: cell channel closed sending ItemUsed — content event lost"
            );
        }

        if consumed_all {
            // Mirror the InventoryItemRemoved emission from `handle_remove_inventory_item`
            // so any cell-side listeners (e.g., bandolier slot reconciliation) see the
            // removal. Sent for all containers — the remove path emits unconditionally
            // on full removal, and any divergence here would silently break listeners
            // for main-bag consumes.
            let _ = cell_tx.send(BaseToCellMsg::InventoryItemRemoved {
                entity_id,
                item_id,
                source_container_id: source.container_id,
            }).await;
        }
    }

    if source.container_id == 3 {
        sync_bandolier_after_inventory_change(
            entity_id, player_id, db_pool, cell_tx, socket, connected, entity_to_addr,
        ).await;
    }
}
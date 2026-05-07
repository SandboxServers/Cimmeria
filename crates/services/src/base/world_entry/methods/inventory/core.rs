use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::super::super::super::helpers::send_to_witness;
use super::super::super::super::ConnectedClientState;
use super::super::vendor::helpers::sync_bandolier_after_inventory_change;
use crate::base::outbox::{self, CellOutboxPayload};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};

/// `pub(crate)` so the duplicate copy in `player_load/core.rs` can be
/// pinned against this one by the SQL drift-guard test
/// `inventory_item_select_matches_player_load_copy_byte_for_byte`. Both
/// paths must produce identical row layouts; if they ever diverge, every
/// downstream `InvItem` consumer breaks in a hard-to-diagnose way.
pub(crate) const INVENTORY_ITEM_SELECT: &str = r#"
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
    stack_size: i32,
    container_id: i32,
}

/// Lighter row for [`handle_remove_inventory_item_by_type`], which needs
/// `item_id` (for the targeted `onRemoveItem` packet) but not the
/// `bound` / `durability` / `charges` metadata.
#[derive(sqlx::FromRow)]
struct InventoryInstanceWithIdRow {
    item_id: i32,
    stack_size: i32,
    container_id: i32,
    slot_id: i32,
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
    let all_items: Vec<InventoryRow> = match sqlx::query_as::<_, InventoryRow>(
        INVENTORY_ITEM_SELECT,
    )
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
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(key, seq, acks, entity_id, method_idx::ON_REMOVE_ITEM, &args)
        },
    )
    .await;
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

/// Resolve an inventory instance's design id and fire the cell-side
/// content-engine `OnItemUse` event. **Does not consume the item.**
///
/// `item_id` here is the inventory **instance id** from the wire (per
/// `SGWInventoryManager.def`'s `useItem` arg). The cell never knows the
/// design id at use-time — base resolves it from `sgw_inventory` and sends
/// it back via `BaseToCellMsg::ItemUsed`. Per-item consumption (if any) is
/// the chain's responsibility via `Action::RemoveItem`, which routes through
/// [`handle_remove_inventory_item_by_type`] — mirrors python's
/// `Inventory.useItem` (`python/cell/Inventory.py:419-432`) which fires
/// `item.use::<typeId>` as a pure event and lets per-mission handlers decide
/// whether to remove the stack (e.g., `FindAmbernol.py:115` calls
/// `removeItemByDesign(19, 1, False)` while radios just don't remove).
///
/// Refusing to fire when the instance isn't owned by this character keeps
/// content events tied to actual inventory state — a malicious client
/// can't spam `useItem(any id)` to trigger arbitrary chain actions.
///
/// Delivery durability: the `ItemUsed` event is enqueued in
/// `cell_event_outbox` first, then dispatched on the in-process channel.
/// `tokio::sync::mpsc::Sender::send().await` only fails when the receiver
/// is dropped (e.g., cell task panicked / shut down) — full channels
/// backpressure rather than error — so this guards specifically against
/// a torn-down receiver. The undelivered row is replayed by the
/// background drainer ([`crate::base::outbox::spawn_drainer`]) once a
/// healthy cell channel is back. The cell-side `BaseToCellMsg::ItemUsed`
/// handler is idempotent — chain conditions self-gate on
/// `step_status = active` so a duplicate fire from a drainer retry is
/// harmless.
pub async fn handle_use_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "UseInventoryItem: no DB pool");
            return;
        }
    };

    // Read-only ownership lookup. No FOR UPDATE — we don't mutate. The
    // simple SELECT also avoids the per-call transaction overhead the
    // pre-#95 consume-on-use code paid.
    let type_id: i32 = match sqlx::query_scalar::<_, i32>(
        "SELECT type_id FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2 LIMIT 1",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(tid)) => tid,
        Ok(None) => {
            tracing::warn!(
                player_id, item_id,
                "UseInventoryItem: instance not found for this character — refusing to fire ItemUsed"
            );
            return;
        }
        Err(e) => {
            tracing::error!(player_id, item_id, "UseInventoryItem: lookup failed: {e}");
            return;
        }
    };

    tracing::info!(
        entity_id,
        player_id,
        item_id,
        type_id,
        target_id,
        "UseInventoryItem: firing ItemUsed (no consumption — chain decides)"
    );

    let payload = CellOutboxPayload::ItemUsed {
        instance_id: item_id,
        type_id,
        target_id,
    };
    let outbox_id = match outbox::enqueue(pool.as_ref(), entity_id, &payload).await {
        Ok(id) => id,
        Err(e) => {
            // Outbox INSERT failed — cannot guarantee delivery, so do NOT
            // attempt the in-process send (a successful send without an
            // outbox row would lose its retry safety net on next failure).
            // The player can re-use the item; ownership lookup above is
            // idempotent.
            tracing::error!(
                entity_id,
                player_id,
                item_id,
                type_id,
                "UseInventoryItem: outbox enqueue failed; ItemUsed not dispatched: {e}"
            );
            return;
        }
    };

    if let Some(cell_tx) = cell_tx {
        outbox::try_dispatch_now(pool.as_ref(), cell_tx, outbox_id, entity_id, payload).await;
    } else {
        tracing::debug!(
            entity_id,
            outbox_id,
            "UseInventoryItem: no cell channel; row left for drainer"
        );
    }
}

/// Resolve a player's first inventory instance with the given design
/// `type_id` and remove `count` from it (delete the row when the stack is
/// fully drained). Used by `Action::RemoveItem` in the cell content
/// executor — chains know item design ids, not instance ids.
///
/// Sends the same client wire-update sequence as
/// [`handle_remove_inventory_item`]: `onRemoveItem` (when the row is
/// deleted) → full inventory refresh → `BaseToCellMsg::InventoryItemRemoved`
/// (when the row is deleted). Bandolier sync runs when the source row was
/// in container 3.
pub async fn handle_remove_inventory_item_by_type(
    entity_id: u32,
    player_id: i32,
    type_id: i32,
    count: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
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
        Ok(_) => {
            let _ = tx.rollback().await;
            tracing::warn!(
                player_id,
                type_id,
                "RemoveInventoryItemByType: no rows changed"
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
            socket,
            connected,
            entity_to_addr,
        )
        .await;
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
            socket,
            connected,
            entity_to_addr,
        )
        .await;
    }
}

//! Core inventory mutation handlers — remove-by-instance, remove-by-type,
//! use-item — plus the shared full-inventory-update broadcaster and
//! `onRemoveItem` UI sync.
//!
//! The three large handlers each own a non-trivial transactional flow and
//! live in sibling modules. `mod.rs` keeps the broadcasters, the row
//! structs they parse into, and the canonical `INVENTORY_ITEM_SELECT`
//! query string that the player-load path drift-guards against.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::helpers::send_to_witness_reliable;
use super::super::super::super::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

mod remove_by_type;
mod remove_instance;
#[cfg(test)]
mod resync_tests;
mod use_instance;
#[cfg(test)]
mod use_instance_tests;

pub use remove_by_type::handle_remove_inventory_item_by_type;
pub use remove_instance::handle_remove_inventory_item;
pub use use_instance::handle_use_inventory_item;

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
pub(super) struct InventoryInstanceRow {
    pub stack_size: i32,
    pub container_id: i32,
}

/// Lighter row for [`handle_remove_inventory_item_by_type`], which needs
/// `item_id` (for the targeted `onRemoveItem` packet) but not the
/// `bound` / `durability` / `charges` metadata.
#[derive(sqlx::FromRow)]
pub(super) struct InventoryInstanceWithIdRow {
    pub item_id: i32,
    pub stack_size: i32,
    pub container_id: i32,
    pub slot_id: i32,
}

/// Send a *full client-side inventory re-init* — the same bundle the
/// world-entry path sends in `map_loaded.rs:336-371`, minus the
/// blueprint and entity-property packets that aren't part of
/// inventory specifically.
///
/// Why this exists separate from [`send_full_inventory_update`]:
/// after `ReanchorPlayer` fires `CREATE_BASE_PLAYER`, the client
/// destroys the pawn actor and instantiates a fresh one with an
/// empty `InventoryComponent`. The new component has no bag list
/// registered, so any subsequent `onUpdateItem` targets a component
/// with no containers and the entries silently fail to slot. The
/// inventory UI stays blank — every existing item is missing, and
/// every new pickup also no-ops because its `onUpdateItem` lands
/// against the same broken state.
///
/// The bundle order mirrors the world-entry sequence:
/// 1. `onBagInfo` — declare the container set on the new
///    `InventoryComponent` so subsequent `onUpdateItem` calls have
///    somewhere to deposit entries.
/// 2. `onActiveSlotUpdate` — seed the bandolier slot indicator from
///    the persisted `bandolier_slot`. Without this the client
///    defaults to slot 1 and the LUA `getActiveSlotForContainer`
///    guard silently drops keypresses for the real persisted slot.
/// 3. `onCashChanged` — naquadah balance. Survives in the DB but
///    the client's `Inventory.cash` is whatever the new pawn
///    defaulted to (zero).
/// 4. `onUpdateItem` — all items, via the shared
///    [`send_full_inventory_update`].
///
/// Cross-world respawn re-runs the full world-entry handshake (the
/// `map_loaded.rs` path above), so it doesn't need this helper —
/// only the same-world `ReanchorPlayer` path does, because the
/// in-place re-anchor intentionally skips the `RESET_ENTITIES` +
/// `onClientMapLoad` reload to preserve client-side kismet state
/// (open doors, completed encounters).
///
/// Called from the `CellToBaseMsg::ListInventoryItems` handler
/// after same-world respawn.
pub async fn send_full_inventory_resync(
    entity_id: u32,
    player_id: i32,
    pool: &Arc<PgPool>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // 1. onBagInfo — declare containers on the fresh InventoryComponent.
    //    Bag set is identity-per-player (every player has the same set
    //    of containers, sized by `BAG_SIZES`); we just need the wire
    //    bytes, no DB lookup required.
    {
        let inv = cimmeria_entity::inventory::Inventory::new(0);
        let bag_info = inv.serialize_bag_info();
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_BAG_INFO,
                    &bag_info,
                )
            },
        )
        .await;
    }

    // 2. onActiveSlotUpdate + 3. onCashChanged — pull from sgw_player.
    //    One round-trip for both. If the row is missing (data corruption
    //    on the player_id), log and skip; the inventory update still
    //    fires so the UI at least shows items.
    let player_meta: Option<(i32, i32)> = match sqlx::query_as::<_, (i32, i32)>(
        "SELECT bandolier_slot, naquadah FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::warn!(
                player_id,
                "send_full_inventory_resync: sgw_player lookup failed: {e}"
            );
            None
        }
    };

    if let Some((bandolier_slot, naquadah)) = player_meta {
        // Wire: `(bag_id:i32, wire_slot:i32)` where wire_slot is the
        // 1-indexed server slot (matches `Bag.py:369` and the live
        // `handle_request_active_slot_change` send shape).
        const CONTAINER_BANDOLIER: i32 = 3;
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&CONTAINER_BANDOLIER.to_le_bytes());
        args.extend_from_slice(&(bandolier_slot + 1).to_le_bytes());
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_ACTIVE_SLOT_UPDATE,
                    &args,
                )
            },
        )
        .await;

        let cash_args = naquadah.to_le_bytes().to_vec();
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CASH_CHANGED,
                    &cash_args,
                )
            },
        )
        .await;
    }

    // 4. onUpdateItem — the existing item-snapshot path.
    let total = send_full_inventory_update(
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
        item_count = total,
        "Sent full inventory resync (onBagInfo + onActiveSlotUpdate + onCashChanged + onUpdateItem)"
    );
}

/// Send full inventory update to player, refreshing all items on the client.
pub async fn send_full_inventory_update(
    entity_id: u32,
    player_id: i32,
    pool: &Arc<PgPool>,
    transport: &Arc<dyn Transport>,
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

    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_UPDATE_ITEM,
                &args,
            )
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
pub(super) async fn send_on_remove_item(
    entity_id: u32,
    item_id: i32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&1u32.to_le_bytes()); // ARRAY<INT32> count
    args.extend_from_slice(&item_id.to_le_bytes());
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_REMOVE_ITEM,
                &args,
            )
        },
    )
    .await;
}

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
use crate::mercury::{build_entity_method_packet, method_idx};

mod remove_by_type;
mod remove_instance;
mod use_instance;

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
            build_entity_method_packet(key, seq, acks, entity_id, method_idx::ON_REMOVE_ITEM, &args)
        },
    )
    .await;
}

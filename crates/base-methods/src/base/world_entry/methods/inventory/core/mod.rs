//! Core inventory mutation handlers — remove-by-instance, remove-by-type,
//! use-item, the native consumable's consume-for-use — plus the shared full-inventory-update broadcaster and
//! `onRemoveItem` UI sync.
//!
//! The three large handlers each own a non-trivial transactional flow and
//! live in sibling modules. `mod.rs` keeps the broadcasters, the row
//! structs they parse into, and the canonical `INVENTORY_ITEM_SELECT`
//! query string that the player-load path drift-guards against.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::plugin::{entity_plugins, InventoryCall, InventoryHookPoint};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::helpers::send_to_witness_reliable;
use super::super::super::super::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

mod access;
#[cfg(test)]
mod access_tests;
mod consume_for_use;
#[cfg(test)]
mod consume_for_use_tests;
#[cfg(test)]
mod crafting_tools_tests;
#[cfg(test)]
mod one_item_select_tests;
mod remove_by_type;
mod remove_instance;
#[cfg(test)]
mod remove_type_read_tests;
#[cfg(test)]
mod resync_tests;
mod use_crafting_item;
#[cfg(test)]
mod use_crafting_item_tests;
mod use_instance;
#[cfg(test)]
mod use_instance_discord_tests;
#[cfg(test)]
mod use_instance_tests;

pub use consume_for_use::handle_consume_item_for_use;
pub use remove_by_type::handle_remove_inventory_item_by_type;
pub use remove_instance::handle_remove_inventory_item;
pub use use_instance::handle_use_inventory_item;

/// The column list and joins every inventory-row read shares, up to (not
/// including) its `WHERE`. The selects below differ only in the filter (and,
/// for the Team and Command vaults, the table), so their row layout cannot
/// drift apart.
macro_rules! inventory_item_select_head {
    () => {
        inventory_item_select_head!("sgw_inventory")
    };
    ($table:literal) => {
        concat!(
            r#"
SELECT inv.item_id, inv.type_id, inv.stack_size, inv.slot_id, inv.container_id,
       inv.bound, inv.durability, inv.charges,
       COALESCE((
           SELECT array_agg(array_position(enum_range(NULL::resources."EAmmoType"), ammo) - 1 ORDER BY ord)
           FROM unnest(ri.ammo_types) WITH ORDINALITY AS ammo_values(ammo, ord)
       ), ARRAY[]::integer[]) AS ammo_type_ids,
       CASE WHEN ri.default_ammo_type IS NULL THEN 0
            ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
       END AS cur_ammo_type_id
FROM "#,
            $table,
            r#" inv
LEFT JOIN resources.items ri ON ri.item_id = inv.type_id
"#
        )
    };
}

// After the macro, so the vault reads can use it.
mod org_vault_items;
pub(crate) use org_vault_items::{org_vault_update_args, send_org_vault_items_via, OrgVaultSend};

/// `pub(crate)` so the duplicate copy in `player_load/core.rs` can be
/// pinned against this one by the SQL drift-guard test
/// `inventory_item_select_matches_player_load_copy_byte_for_byte`. Both
/// paths must produce identical row layouts; if they ever diverge, every
/// downstream `InvItem` consumer breaks in a hard-to-diagnose way.
///
/// Container 18 (`INV_AUCTION`) is excluded: it holds the rows the player
/// has listed on the Black Market (BM-02), which are server-held and never
/// shown to the client.
pub(crate) const INVENTORY_ITEM_SELECT: &str = concat!(
    inventory_item_select_head!(),
    "WHERE inv.character_id = $1 AND inv.container_id <> 18\n",
    "ORDER BY inv.container_id, inv.slot_id\n",
);

/// One inventory row, by instance id, and only if the player owns it: the
/// same row layout as [`INVENTORY_ITEM_SELECT`] (shared head), filtered in
/// SQL so a single-item read costs one indexed row, not the whole bag set.
/// `$1` is the player, `$2` the `item_id`. A listed row (container 18) is
/// not the player's to see, so a refused move's snap-back resync of a
/// remembered listed id sends nothing.
pub(crate) const INVENTORY_ONE_ITEM_SELECT: &str = concat!(
    inventory_item_select_head!(),
    "WHERE inv.character_id = $1 AND inv.item_id = $2 AND inv.container_id <> 18\n",
);

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
    /// The design id, read only so the log lines can name the item (Rule 6).
    /// `Option` so a log-only read can never fail a removal: the column is
    /// declared nullable on `sgw_inventory` (the inherited `NOT NULL` on
    /// `sgw_inventory_base` rules NULL out today).
    pub type_id: Option<i32>,
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
    // One round-trip for the three `sgw_player` fields the bundle needs:
    // `bank_slots` (container 17's size in onBagInfo), `bandolier_slot`
    // (onActiveSlotUpdate) and `naquadah` (onCashChanged). If the row is
    // missing (data corruption on the player_id), log and skip those two;
    // onBagInfo falls back to the default vault size and the inventory
    // update still fires so the UI at least shows items.
    let player_meta: Option<(i32, i32, i16)> = match sqlx::query_as::<_, (i32, i32, i16)>(
        "SELECT bandolier_slot, naquadah, bank_slots FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::warn!(
                player_id,
                player_name = known_names::player_name(player_id),
                "send_full_inventory_resync: sgw_player lookup failed: {e}"
            );
            None
        }
    };

    // 1. onBagInfo — declare containers on the fresh InventoryComponent.
    //    Every player has the same container set, sized by `BAG_SIZES`,
    //    except the personal vault (17), which is this player's
    //    `bank_slots` (D-BV06).
    {
        let bank_slots = player_meta.map_or(
            cimmeria_entity::inventory::BANK_SLOTS_DEFAULT,
            |(_, _, bank_slots)| i32::from(bank_slots),
        );
        let inv = cimmeria_entity::inventory::Inventory::new(0).with_bank_slots(bank_slots);
        let bag_info = inv.serialize_bag_info();
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_BAG_INFO,
                    &bag_info,
                    version,
                )
            },
        )
        .await;
    }

    // 2. onActiveSlotUpdate + 3. onCashChanged.
    if let Some((bandolier_slot, naquadah, _)) = player_meta {
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
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_ACTIVE_SLOT_UPDATE,
                    &args,
                    version,
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
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CASH_CHANGED,
                    &cash_args,
                    version,
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
    let player_label = known_names::player_name(player_id);
    tracing::info!(
        entity_id,
        entity_name = player_label,
        player_id,
        player_name = player_label,
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
            let player_label = known_names::player_name(player_id);
            tracing::error!(
                    entity_id,
                    entity_name = player_label,
                    player_id,
                    player_name = player_label,
                    "send_full_inventory_update: query failed, refusing to broadcast empty inventory: {e}"
                );
            return 0;
        }
    };

    send_update_item(entity_id, &all_items, transport, connected, entity_to_addr).await;

    // Every inventory commit ends in this resync, so it is where the base
    // plugins see the new inventory (#962 step 5): the crafting options
    // learn that a Field Crafting Tool entered or left the crafting bag. No
    // second query: the rows above carry the container.
    let plugins = entity_plugins(connected, entity_to_addr, entity_id);
    let rows: Vec<(i32, i32, i32)> = all_items
        .iter()
        .map(|r| (r.item_id, r.type_id, r.container_id))
        .collect();
    plugins
        .run_inventory_hook(
            InventoryHookPoint::AfterFullInventoryUpdate,
            InventoryCall {
                entity_id,
                player_id,
                pool,
                rows: &rows,
                transport,
                connected,
                entity_to_addr,
            },
        )
        .await;

    all_items.len()
}

/// Send `onUpdateItem` for one inventory instance only, read through
/// `executor` so the caller can read it inside its own transaction, under a
/// lock it holds. Returns `false`, and sends nothing, when the player has no
/// such row (a forged or stale `item_id`) or the read fails.
///
/// The move refusal uses this to snap one dragged item back. A full
/// snapshot would also carry every other row as of the read, and a grant
/// committing between that read and the send would then be hidden on the
/// client by the older snapshot. With one row, only a write to that same row
/// can race the send, and the refusal holds the row's `FOR UPDATE` lock
/// across it (`move_/container_policy.rs`, `take_move_lock`).
pub(crate) async fn send_inventory_item_update_via<'c, E>(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    executor: E,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool
where
    E: sqlx::PgExecutor<'c>,
{
    // `moveItem` is client-callable, so a refusal must cost one row however
    // big the inventory is: the owner check and the item filter are in SQL.
    let row = match sqlx::query_as::<_, InventoryRow>(INVENTORY_ONE_ITEM_SELECT)
        .bind(player_id)
        .bind(item_id)
        .fetch_optional(executor)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            let player_label = known_names::player_name(player_id);
            tracing::error!(
                entity_id,
                entity_name = player_label,
                player_id,
                player_name = player_label,
                item_id, // nt:id-only instance id, type unread yet
                "send_inventory_item_update_via: query failed: {e}"
            );
            return false;
        }
    };
    let Some(row) = row else {
        return false;
    };
    send_update_item(
        entity_id,
        std::slice::from_ref(&row),
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    true
}

/// `onUpdateItem(ARRAY<InvItem>)` for `rows`, to the player's own client.
async fn send_update_item(
    entity_id: u32,
    rows: &[InventoryRow],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let args = update_item_args(rows);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_UPDATE_ITEM,
                &args,
                version,
            )
        },
    )
    .await;
}

/// The `onUpdateItem(ARRAY<InvItem>)` args for `rows`.
fn update_item_args(rows: &[InventoryRow]) -> Vec<u8> {
    let mut args = Vec::with_capacity(4 + rows.len() * 48);
    args.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    for row in rows {
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
    args
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
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_REMOVE_ITEM,
                &args,
                version,
            )
        },
    )
    .await;
}

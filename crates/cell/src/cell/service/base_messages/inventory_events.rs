//! Inventory-event `BaseToCellMsg` handlers — the cell-side reactions to
//! base-applied item mutations: move (bandolier-equip content event), remove,
//! grant, and use (`OnItemUse` content trigger). Extracted from
//! `base_messages/mod.rs` as a pure code move.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::content;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Handle `BaseToCellMsg::InventoryItemMoveApplied`.
///
/// Bandolier state is re-synced via SyncBandolierItems; this handler also
/// fires the `OnItemEquipped` content event when an item lands in the
/// bandolier from a non-bandolier container, so quest chains keyed on
/// `item_equipped::<type_id>` can advance (mission 622 pistol, mission
/// 641 P90).
pub(super) async fn handle_inventory_item_move_applied(
    entity_id: u32,
    item_id: i32,
    type_id: i32,
    source_container_id: i32,
    target_container_id: i32,
    swapped_item_id: Option<i32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    tracing::debug!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        item_id,
        item_type_id = type_id,
        item_name = cimmeria_names::book().item(type_id),
        source_container_id,
        source_container_name = cimmeria_names::book().container(source_container_id),
        target_container_id,
        target_container_name = cimmeria_names::book().container(target_container_id),
        swapped_item_id = ?swapped_item_id, // nt:id-only instance id, type not sent
        "Item moved in inventory"
    );

    const INV_BANDOLIER: i32 = 3;
    if target_container_id == INV_BANDOLIER && source_container_id != INV_BANDOLIER {
        let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
            Some(pid) => pid,
            None => {
                tracing::warn!(
                    entity_id,
                    entity_name = space_mgr.entity_label(entity_id),
                    item_type_id = type_id,
                    item_name = cimmeria_names::book().item(type_id),
                    "InventoryItemMoveApplied: entity has no player_id — equip event dropped"
                );
                return;
            }
        };
        content::fire_item_equipped(entity_id, player_id, type_id, engine, tx, space_mgr).await;
    }
}

/// Handle `BaseToCellMsg::InventoryItemRemoved`.
pub(super) fn handle_inventory_item_removed(
    entity_id: u32,
    item_id: i32,
    source_container_id: i32,
    space_mgr: &SpaceManager,
) {
    tracing::debug!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        item_id, // nt:id-only instance id, type not sent
        source_container_id,
        source_container_name = cimmeria_names::book().container(source_container_id),
        "Item removed from inventory"
    );
}

/// Handle `BaseToCellMsg::InventoryItemGranted`.
///
/// `item_id` on this message is the item's design (type) id — every sender
/// (`grant::persist`, `vendor::purchase`, the crafting transaction) fills it
/// from the grant's type id. It is logged as `design_id` so the row joins the
/// cell's `Player looted item` and the base's `inventory`
/// `grant_container_chosen` row; the old `item_id` field name stays for
/// existing saved queries. `item_type_id` is Rule 6's key for the same
/// number, the one the joins move to once the loot rows are swept.
pub(super) fn handle_inventory_item_granted(
    entity_id: u32,
    item_id: i32,
    container_id: i32,
    slot_id: i32,
    quantity: i32,
    space_mgr: &SpaceManager,
) {
    let identity = space_mgr.player_identity(entity_id);
    tracing::debug!(
        account_id = identity.account_id,
        account_name = identity.account_name,
        player_id = identity.player_id,
        player_name = identity.player_name,
        entity_id,
        entity_name = identity.player_name,
        item_id,
        item_type_id = item_id,
        design_id = item_id, // nt:id-only join key, item_name names it
        item_name = cimmeria_names::book().item(item_id),
        container_id,
        container_name = cimmeria_names::book().container(container_id),
        slot_id, // nt:id-only slot index, unnamed
        quantity,
        "Item granted to player"
    );
}

/// Handle `BaseToCellMsg::ItemUsed`.
pub(super) async fn handle_item_used(
    entity_id: u32,
    instance_id: i32,
    type_id: i32,
    target_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // Base verified ownership and forwarded the use event. Fire
    // `OnItemUse` so any chain conditioned on `item_use::<type_id>`
    // can run. The chain decides whether to consume (via
    // `Action::RemoveItem`) — base does NOT consume before this
    // message, the historical comment about a "consumption tx"
    // pre-dated the chain-decides-consumption design.
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                item_type_id = type_id,
                item_name = cimmeria_names::book().item(type_id),
                "ItemUsed: entity has no player_id — content event dropped"
            );
            return;
        }
    };
    tracing::debug!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        player_id,
        player_name = space_mgr.entity_label(entity_id),
        instance_id, // nt:id-only instance row of item_type_id
        item_type_id = type_id,
        item_name = cimmeria_names::book().item(type_id),
        target_id,
        target_name = u32::try_from(target_id)
            .ok()
            .and_then(|t| space_mgr.entity_label(t)),
        "ItemUsed: firing OnItemUse"
    );
    content::fire_item_use(
        entity_id,
        player_id,
        instance_id,
        type_id,
        engine,
        tx,
        space_mgr,
    )
    .await;
}

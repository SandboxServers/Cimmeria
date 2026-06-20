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
    tracing::debug!(entity_id, item_id, type_id, source = source_container_id, target = target_container_id, swapped_item_id = ?swapped_item_id, "Item moved in inventory");

    const INV_BANDOLIER: i32 = 3;
    if target_container_id == INV_BANDOLIER && source_container_id != INV_BANDOLIER {
        let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
            Some(pid) => pid,
            None => {
                tracing::warn!(
                    entity_id,
                    type_id,
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
) {
    tracing::debug!(
        entity_id,
        item_id,
        source = source_container_id,
        "Item removed from inventory"
    );
}

/// Handle `BaseToCellMsg::InventoryItemGranted`.
pub(super) fn handle_inventory_item_granted(
    entity_id: u32,
    item_id: i32,
    container_id: i32,
    slot_id: i32,
    quantity: i32,
) {
    tracing::debug!(
        entity_id,
        item_id,
        container_id,
        slot_id,
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
                type_id,
                "ItemUsed: entity has no player_id — content event dropped"
            );
            return;
        }
    };
    tracing::debug!(
        entity_id,
        player_id,
        instance_id,
        type_id,
        target_id,
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

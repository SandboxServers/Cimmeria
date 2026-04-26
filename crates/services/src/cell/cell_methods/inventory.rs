//! SGWInventoryManager interface exposed CellMethods (indices 36–42).

use tokio::sync::mpsc;
use cimmeria_content_engine::chain::ChainEngine;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub const REMOVE_ITEM: u16 = 36;
pub const LIST_ITEMS: u16 = 37;
pub const MOVE_ITEM: u16 = 38;
pub const USE_ITEM: u16 = 39;
pub const REPAIR_ITEM_REQUEST: u16 = 40;
pub const REQUEST_ACTIVE_SLOT_CHANGE: u16 = 41;
pub const REQUEST_AMMO_CHANGE: u16 = 42;

/// Fire a weapon attack event for an equipped item.
/// Called after a ranged weapon attack is launched.
pub async fn fire_equipped_weapon_attack_event(
    entity_id: u32,
    target_id: i32,
    is_ranged: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::debug!(
        entity_id,
        target_id,
        is_ranged,
        "fireEquippedWeaponAttackEvent"
    );
}

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        REMOVE_ITEM => {
            if args.len() >= 6 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let quantity = i16::from_le_bytes([args[4], args[5]]);
                tracing::info!(entity_id, item_id, quantity, "UNIMPLEMENTED: removeItem");
            }
            true
        }
        LIST_ITEMS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: listItems");
            true
        }
        MOVE_ITEM => {
            if args.len() >= 16 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_bag = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_slot = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let quantity = i32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::info!(entity_id, item_id, target_bag, target_slot, quantity, "UNIMPLEMENTED: moveItem");
            }
            true
        }
        USE_ITEM => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, item_id, target_id, "useItem");

                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id).unwrap_or(0);
                crate::cell::content::fire_item_use(
                    entity_id, player_id, item_id, engine, tx, space_mgr,
                ).await;
            }
            true
        }
        REPAIR_ITEM_REQUEST => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let repair_ratio = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, item_id, repair_ratio, "UNIMPLEMENTED: repairItemRequest");
            }
            true
        }
        REQUEST_ACTIVE_SLOT_CHANGE => {
            if args.len() >= 8 {
                let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let slot_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, bag_id, slot_id, "UNIMPLEMENTED: requestActiveSlotChange");
            }
            true
        }
        REQUEST_AMMO_CHANGE => {
            if args.len() >= 8 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ammo_type = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, item_id, ammo_type, "UNIMPLEMENTED: requestAmmoChange");
            }
            true
        }
        _ => false,
    }
}

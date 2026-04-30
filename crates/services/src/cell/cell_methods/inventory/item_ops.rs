//! Simple inventory operations that forward to the base service:
//! remove / list / move / use / repair. These handlers parse args, look up
//! the player_id, and send a CellToBaseMsg — they do not touch entity state
//! directly.

use tokio::sync::mpsc;
use cimmeria_content_engine::chain::ChainEngine;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Resolve the player_id for an inventory op. We refuse to silently default
/// to 0 (would either no-op or hit a sentinel row).
pub(super) fn resolve_player_id(
    entity_id: u32,
    op: &str,
    space_mgr: &SpaceManager,
) -> Option<i32> {
    match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(id) => Some(id),
        None => {
            tracing::warn!(entity_id, op, "inventory op dropped: entity has no player_id");
            None
        }
    }
}

pub(super) async fn handle_remove_item(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if args.len() >= 6 {
        let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        let quantity = i16::from_le_bytes([args[4], args[5]]) as i32;
        tracing::debug!(entity_id, item_id, quantity, "removeItem");
        if let Some(player_id) = resolve_player_id(entity_id, "removeItem", space_mgr) {
            if let Err(e) = tx.send(CellToBaseMsg::RemoveInventoryItem {
                entity_id, player_id, item_id, quantity,
            }).await {
                tracing::error!(
                    entity_id, player_id, item_id, quantity, error = %e,
                    "RemoveInventoryItem send to base failed"
                );
            }
        }
    } else {
        tracing::warn!(entity_id, args_len = args.len(), "removeItem: truncated args");
    }
}

pub(super) async fn handle_list_items(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    tracing::debug!(entity_id, "listItems");
    if let Some(player_id) = resolve_player_id(entity_id, "listItems", space_mgr) {
        if let Err(e) = tx.send(CellToBaseMsg::ListInventoryItems { entity_id, player_id }).await {
            tracing::error!(
                entity_id, player_id, error = %e,
                "ListInventoryItems send to base failed"
            );
        }
    }
}

pub(super) async fn handle_move_item(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if args.len() >= 16 {
        let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        let target_container_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
        let target_slot_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
        let quantity = i32::from_le_bytes([args[12], args[13], args[14], args[15]]);
        tracing::debug!(entity_id, item_id, target_container_id, target_slot_id, quantity, "moveItem");
        if let Some(player_id) = resolve_player_id(entity_id, "moveItem", space_mgr) {
            if let Err(e) = tx.send(CellToBaseMsg::MoveInventoryItem {
                entity_id, player_id, item_id,
                target_container_id, target_slot_id, quantity,
            }).await {
                tracing::error!(
                    entity_id, player_id, item_id,
                    target_container_id, target_slot_id, quantity,
                    error = %e,
                    "MoveInventoryItem send to base failed"
                );
            }
        }
    } else {
        tracing::warn!(entity_id, args_len = args.len(), "moveItem: truncated args");
    }
}

pub(super) async fn handle_use_item(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if args.len() >= 8 {
        let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
        tracing::info!(entity_id, item_id, target_id, "useItem");

        if let Some(player_id) = resolve_player_id(entity_id, "useItem", space_mgr) {
            crate::cell::content::fire_item_use(
                entity_id, player_id, item_id, engine, tx, space_mgr,
            ).await;
        }
    } else {
        tracing::warn!(entity_id, args_len = args.len(), "useItem: truncated args");
    }
}

pub(super) async fn handle_repair_item_request(entity_id: u32, args: &[u8]) {
    if args.len() >= 8 {
        let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        let repair_ratio = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
        tracing::info!(entity_id, item_id, repair_ratio, "UNIMPLEMENTED: repairItemRequest");
    }
}

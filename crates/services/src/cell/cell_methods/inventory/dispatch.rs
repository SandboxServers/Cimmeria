//! Top-level dispatch for SGWInventoryManager method indices (36–42).

use tokio::sync::mpsc;
use cimmeria_content_engine::chain::ChainEngine;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::bandolier::{handle_request_active_slot_change, handle_request_ammo_change};
use super::constants::{
    LIST_ITEMS, MOVE_ITEM, REMOVE_ITEM, REPAIR_ITEM_REQUEST,
    REQUEST_ACTIVE_SLOT_CHANGE, REQUEST_AMMO_CHANGE, USE_ITEM,
};
use super::item_ops::{
    handle_list_items, handle_move_item, handle_remove_item, handle_repair_item_request,
    handle_use_item,
};

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
            handle_remove_item(entity_id, args, tx, space_mgr).await;
            true
        }
        LIST_ITEMS => {
            handle_list_items(entity_id, tx, space_mgr).await;
            true
        }
        MOVE_ITEM => {
            handle_move_item(entity_id, args, tx, space_mgr).await;
            true
        }
        USE_ITEM => {
            handle_use_item(entity_id, args, tx, space_mgr, engine).await;
            true
        }
        REPAIR_ITEM_REQUEST => {
            handle_repair_item_request(entity_id, args).await;
            true
        }
        REQUEST_ACTIVE_SLOT_CHANGE => {
            handle_request_active_slot_change(entity_id, args, tx, space_mgr).await;
            true
        }
        REQUEST_AMMO_CHANGE => {
            handle_request_ammo_change(entity_id, args, tx, space_mgr).await;
            true
        }
        _ => false,
    }
}

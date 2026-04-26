use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        TRAIN_ABILITY => {
            if args.len() >= 4 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, ability_id, "UNIMPLEMENTED: trainAbility");
            }
            true
        }

        PURCHASE_ITEMS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: purchaseItems");
            true
        }

        SELL_ITEMS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: sellItems");
            true
        }

        BUYBACK_ITEMS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: buybackItems");
            true
        }

        REPAIR_ITEMS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: repairItems");
            true
        }

        RECHARGE_ITEMS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: rechargeItems");
            true
        }

        _ => false,
    }
}

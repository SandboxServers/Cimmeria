use tokio::sync::mpsc;
use cimmeria_content_engine::chain::ChainEngine;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        super::constants::CALL_FOR_AID..=super::constants::RESET_MY_ABILITIES => {
            super::combat::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await
        }
        super::constants::WHO..=super::constants::INITIAL_RESPONSE => {
            super::interaction::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await
        }
        super::constants::TRAIN_ABILITY..=super::constants::RECHARGE_ITEMS => {
            super::vendor::dispatch(entity_id, method_index, args, tx, space_mgr).await
        }
        super::constants::PET_INVOKE_ABILITY..=super::constants::PET_CHANGE_STANCE => {
            super::social::dispatch(entity_id, method_index, args, tx, space_mgr).await
        }
        super::constants::SET_AUTO_CYCLE..=super::constants::UPDATE_SYSTEM_OPTIONS => {
            super::world::dispatch(entity_id, method_index, args, tx, space_mgr, engine).await
        }
        super::constants::ORG_CREATION..=super::constants::CANCEL_MOVIE => {
            if method_index >= super::constants::ORG_CREATION && method_index <= super::constants::SPEND_APPLIED_SCIENCE_POINTS {
                super::social::dispatch(entity_id, method_index, args, tx, space_mgr).await
            } else if method_index >= super::constants::CRAFT && method_index <= super::constants::RESPEC_CRAFTING {
                super::crafting::dispatch(entity_id, method_index, args, tx, space_mgr).await
            } else {
                super::social::dispatch(entity_id, method_index, args, tx, space_mgr).await
            }
        }
        _ => false,
    }
}

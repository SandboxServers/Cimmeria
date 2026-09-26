//! SGWAbilityManager interface exposed CellMethods (indices 2–4).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

pub use cimmeria_wire::cell::cell_methods::ability_manager::{
    CONFIRMATION_RESPONSE, TOGGLE_COMBAT_DEBUG, TOGGLE_COMBAT_VERBOSE_DEBUG,
};

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        TOGGLE_COMBAT_DEBUG | TOGGLE_COMBAT_VERBOSE_DEBUG => {
            tracing::debug!(entity_id, method_index, "Debug toggle (no-op)");
            true
        }
        CONFIRMATION_RESPONSE => {
            if args.len() >= 5 {
                let effect_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let accepted = args[4] != 0;
                tracing::debug!(entity_id, effect_id, accepted, "confirmationResponse");
            }
            true
        }
        _ => false,
    }
}

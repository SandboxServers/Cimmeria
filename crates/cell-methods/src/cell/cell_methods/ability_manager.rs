//! SGWAbilityManager interface exposed CellMethods (indices 2–4).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use cimmeria_cell_world::cell::combat_debug::commands::{toggle_from_cell_method, Toggle};

pub use cimmeria_wire::cell::cell_methods::ability_manager::{
    CONFIRMATION_RESPONSE, TOGGLE_COMBAT_DEBUG, TOGGLE_COMBAT_VERBOSE_DEBUG,
};

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        // Crafted callers only: the stock client sends the GM twins 170 and
        // 171 instead (AB-N1). GM-gated in `gm_gate`.
        TOGGLE_COMBAT_DEBUG | TOGGLE_COMBAT_VERBOSE_DEBUG => {
            let (cmd, which) = if method_index == TOGGLE_COMBAT_DEBUG {
                ("toggleCombatDebug", Toggle::Combat)
            } else {
                ("toggleCombatVerboseDebug", Toggle::Verbose)
            };
            toggle_from_cell_method(tx, space_mgr, entity_id, method_index, cmd, which).await;
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

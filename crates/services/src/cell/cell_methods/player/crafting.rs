use tokio::sync::mpsc;
use crate::cell::space_manager::SpaceManager;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        CRAFT => {
            if args.len() >= 4 {
                let craft_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, craft_id, "UNIMPLEMENTED: craft");
            } else {
                tracing::warn!(entity_id, args_len = args.len(), "craft: malformed/truncated args");
            }
            true
        }

        RESEARCH => {
            tracing::info!(entity_id, "UNIMPLEMENTED: research");
            true
        }

        REVERSE_ENGINEER => {
            tracing::info!(entity_id, "UNIMPLEMENTED: reverseEngineer");
            true
        }

        ALLOYING => {
            if args.len() >= 4 {
                let craft_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, craft_id, "UNIMPLEMENTED: alloying");
            } else {
                tracing::warn!(entity_id, args_len = args.len(), "alloying: malformed/truncated args");
            }
            true
        }

        RESPEC_CRAFTING => {
            tracing::info!(entity_id, "UNIMPLEMENTED: respecCrafting");
            true
        }

        _ => false,
    }
}

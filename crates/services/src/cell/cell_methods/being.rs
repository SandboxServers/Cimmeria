//! SGWBeing interface exposed CellMethods (indices 0–1).

use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Set current target entity.
pub const SET_TARGET_ID: u16 = 0;
/// Set movement type (walk/run/sprint).
pub const SET_MOVEMENT_TYPE: u16 = 1;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SET_TARGET_ID => {
            if args.len() >= 4 {
                let target_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, target_id, "setTargetID");
            }
            true
        }
        SET_MOVEMENT_TYPE => {
            if !args.is_empty() {
                let movement_type = args[0];
                tracing::debug!(entity_id, movement_type, "setMovementType");
            }
            true
        }
        _ => false,
    }
}

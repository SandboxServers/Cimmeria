//! Missionary interface exposed CellMethods (indices 52–54).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

pub const ABANDON_MISSION: u16 = 52;
pub const SHARE_MISSION: u16 = 53;
pub const SHARE_MISSION_RESPONSE: u16 = 54;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        ABANDON_MISSION => {
            if args.len() >= 4 {
                let mission_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mission_id, "abandonMission");
                crate::cell::missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
            }
            true
        }
        SHARE_MISSION => {
            if args.len() >= 4 {
                let mission_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, mission_id, "UNIMPLEMENTED: shareMission");
            }
            true
        }
        SHARE_MISSION_RESPONSE => {
            if !args.is_empty() {
                let choice = args[0] as i8;
                tracing::info!(entity_id, choice, "UNIMPLEMENTED: shareMissionResponse");
            }
            true
        }
        _ => false,
    }
}

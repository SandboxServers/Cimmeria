//! Missionary interface exposed CellMethods (indices 52–54).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

pub use cimmeria_wire::cell::cell_methods::missionary::{
    ABANDON_MISSION, SHARE_MISSION, SHARE_MISSION_RESPONSE,
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
        ABANDON_MISSION => {
            if args.len() >= 4 {
                let mission_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mission_id, "abandonMission");
                // H54: only on a real removal. A client can send
                // `abandonMission` for anything; repainting an offer for a
                // mission the player never held would be a free re-trigger.
                if crate::cell::missions::abandon_mission(entity_id, mission_id, tx, space_mgr)
                    .await
                {
                    let player_id = space_mgr
                        .get_entity(entity_id)
                        .and_then(|e| e.player_id)
                        .unwrap_or(0);
                    crate::cell::content::fire_mission_abandoned(
                        entity_id, player_id, mission_id, engine, tx, space_mgr,
                    )
                    .await;
                }
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

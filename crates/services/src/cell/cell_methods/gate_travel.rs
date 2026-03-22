//! GateTravel interface exposed CellMethods (index 35).

use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub const ON_DIAL_GATE: u16 = 35;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        ON_DIAL_GATE => {
            if args.len() >= 8 {
                let target_address_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let source_address_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, target_address_id, source_address_id, "onDialGate");
                crate::cell::gate_travel::handle_dial_gate(
                    entity_id, target_address_id, source_address_id, tx, space_mgr,
                ).await;
            }
            true
        }
        _ => false,
    }
}

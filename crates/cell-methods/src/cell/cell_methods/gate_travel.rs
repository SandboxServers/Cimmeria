//! GateTravel interface exposed CellMethods (index 35).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

pub use cimmeria_wire::cell::cell_methods::gate_travel::ON_DIAL_GATE;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        ON_DIAL_GATE => {
            if args.len() >= 8 {
                let target_address_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let source_address_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(
                    entity_id,
                    target_address_id,
                    source_address_id,
                    "onDialGate"
                );
                // Nothing to forward here: `handle_dial_gate` tells the
                // player itself on every refusal (#727, `gate_travel::
                // dial_feedback`) with a `CHAN_FEEDBACK` line. The client does
                // have a dial-failure surface — `onDHDReply` (client method
                // 100, WSTRING) — but where it renders is unverified, so the
                // chat line is the one sent.
                let _dialed = crate::cell::gate_travel::handle_dial_gate(
                    entity_id,
                    target_address_id,
                    source_address_id,
                    tx,
                    space_mgr,
                    engine,
                )
                .await;
            }
            true
        }
        _ => false,
    }
}

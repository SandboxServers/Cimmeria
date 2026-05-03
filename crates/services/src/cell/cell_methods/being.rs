//! SGWBeing interface exposed CellMethods (indices 0–1).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Set current target entity.
pub const SET_TARGET_ID: u16 = 0;
/// Set movement type (walk/run/sprint).
pub const SET_MOVEMENT_TYPE: u16 = 1;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SET_TARGET_ID => {
            if args.len() >= 4 {
                let target_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, target_id, "setTargetID");

                // Send onTargetUpdate (client method 16) back to the player
                // so the client knows the target is set and enables auto-attack.
                let mut reply = Vec::with_capacity(4);
                reply.extend_from_slice(&target_id.to_le_bytes());
                let _ = tx
                    .send(CellToBaseMsg::EntityMethodCall {
                        entity_id,
                        method_index: 16, // onTargetUpdate (SGWBeing interface)
                        args: reply,
                    })
                    .await;

                // Also notify witnesses so they see who we're targeting
                let witnesses = space_mgr.get_witnesses_of(entity_id);
                if !witnesses.is_empty() {
                    let mut witness_args = Vec::with_capacity(4);
                    witness_args.extend_from_slice(&target_id.to_le_bytes());
                    for witness_id in witnesses {
                        let _ = tx
                            .send(CellToBaseMsg::WitnessEntityMethod {
                                witness_id,
                                entity_id,
                                method_index: 16,
                                args: witness_args.clone(),
                            })
                            .await;
                    }
                }
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

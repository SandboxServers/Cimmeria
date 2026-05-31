use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        PET_INVOKE_ABILITY => {
            if args.len() >= 12 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ability_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(
                    entity_id,
                    pet_entity_id,
                    ability_id,
                    target_id,
                    "UNIMPLEMENTED: petInvokeAbility"
                );
            }
            true
        }

        PET_ABILITY_TOGGLE => {
            if args.len() >= 9 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let ability_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let toggle = args[8] as i8;
                tracing::info!(
                    entity_id,
                    pet_entity_id,
                    ability_id,
                    toggle,
                    "UNIMPLEMENTED: petAbilityToggle"
                );
            }
            true
        }

        PET_CHANGE_STANCE => {
            if args.len() >= 5 {
                let pet_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let stance = args[4] as i8;
                tracing::info!(
                    entity_id,
                    pet_entity_id,
                    stance,
                    "UNIMPLEMENTED: petChangeStance"
                );
            }
            true
        }

        ORG_CREATION => {
            tracing::info!(entity_id, "UNIMPLEMENTED: onOrganizationCreation");
            true
        }

        SPEND_APPLIED_SCIENCE_POINTS => {
            if args.len() >= 4 {
                let discipline_seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    discipline_seq_id,
                    "UNIMPLEMENTED: spendAppliedSciencePoints"
                );
            }
            true
        }

        CLIENT_CHALLENGE_RESPONSE => {
            if args.len() >= 4 {
                let challenge = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(
                    entity_id,
                    challenge,
                    "UNIMPLEMENTED: onClientChallengeResponse"
                );
            }
            true
        }

        SEND_DUEL_RESPONSE => {
            if !args.is_empty() {
                let response = args[0] as i8;
                tracing::info!(entity_id, response, "UNIMPLEMENTED: sendDuelResponse");
            }
            true
        }

        DUEL_FORFEIT => {
            tracing::info!(entity_id, "UNIMPLEMENTED: duelForfeit");
            true
        }

        // TRADE_REQUEST / TRADE_REQUEST_CANCEL / TRADE_UPDATE_PROPOSAL /
        // TRADE_LOCK_STATE (104..=107) used to be stubbed here as
        // UNIMPLEMENTED log lines. They are now routed by the outer
        // dispatcher to `cell_methods::player::trade::dispatch` and
        // implemented for real. If you see a method in the 104..=107
        // range reach this catch-all, the routing in `dispatch.rs`
        // regressed — the trade sub-range arm is missing or
        // mis-ordered.
        CANCEL_MOVIE => {
            tracing::info!(entity_id, "UNIMPLEMENTED: cancelMovie");
            true
        }

        _ => false,
    }
}

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    // Unused since ORG_CREATION left for the org plugin; kept so every
    // SGWPlayer sub-dispatcher has the same shape.
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        // PET_INVOKE_ABILITY / PET_ABILITY_TOGGLE / PET_CHANGE_STANCE (88..=90)
        // used to be stubbed here. They are routed by the outer dispatcher
        // to the pets plugin (`cimmeria-cell-pets`, #962). This module
        // no longer handles them; `social_submodule_does_not_handle_pet_methods`
        // in `dispatch.rs` is the guard.
        // ORG_CREATION (94, `onOrganizationCreation`) is not handled here:
        // the org plugin registers it (`cimmeria-cell-org`, #962 step 3),
        // and the cell router asks the plugin registry first. An arm here
        // would hide a missing plugin; `plugin_owned_methods_are_not_routed_here`
        // in `dispatch.rs` is the guard.

        // No arm for SPEND_APPLIED_SCIENCE_POINTS here — index 95 is owned
        // by the crafting submodule. The outer dispatcher's
        // `SPEND_APPLIED_SCIENCE_POINTS..=RESPEC_CRAFTING` range routes 95
        // to `crafting::dispatch` before social ever sees it.
        //
        // A shadow stub used to live at this location: it was a legacy
        // artifact from before crafting got its own submodule, when every
        // post-ORG_CREATION method either landed in social or was a no-op.
        // After crafting was extracted, the shadow stayed behind and would
        // silently re-handle 95 if the outer router were ever narrowed back
        // to `CRAFT..=RESPEC_CRAFTING` — making `assert!(handled)`-style
        // routing tests theatre because both arms returned `true`. The
        // `route_index_95_must_go_to_crafting_not_social` test in
        // `dispatch.rs` is the regression guard.
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

        // SEND_DUEL_RESPONSE / DUEL_FORFEIT (102..=103) are not handled
        // here: the duel plugin registers them (`cimmeria-cell-duel`, #962),
        // and the cell router asks the plugin registry first. An arm here
        // would hide a missing plugin; `plugin_owned_methods_are_not_routed_here`
        // in `dispatch.rs` is the guard.

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

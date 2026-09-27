use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use cimmeria_wire::cell::cell_methods::organization::decode_on_organization_creation;

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
            // `onOrganizationCreation(WSTRING aOrganizationName)`. The name
            // used to be dropped (audit A-02); the pending-creation check,
            // D-ORG10 validation and the forward to the base are ORG-05's.
            match decode_on_organization_creation(args) {
                Ok(name) => tracing::info!(
                    target: "org",
                    event = "org.cell_method_unimplemented",
                    entity_id,
                    method_index,
                    method = "onOrganizationCreation",
                    text_units = name.encode_utf16().count(),
                    "UNIMPLEMENTED: onOrganizationCreation"
                ),
                Err(e) => tracing::warn!(
                    target: "org",
                    event = "org.cell_method_malformed",
                    entity_id,
                    method_index,
                    reason = e.reason(),
                    error = %e,
                    "organization cell method payload did not decode"
                ),
            }
            true
        }

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

#[cfg(test)]
mod tests {
    use tracing::Level;

    use super::*;
    use crate::test_support::{make_space_manager_with_player, LogCapture};

    /// CM 94 carries only the name (audit A-09). It used to be ignored
    /// entirely; the dispatcher now decodes it: "SG-1" is four units.
    #[tokio::test]
    async fn org_creation_decodes_the_name() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        let args = [4, 0, 0, 0, 0x53, 0, 0x47, 0, 0x2D, 0, 0x31, 0];
        assert!(dispatch(1, ORG_CREATION, &args, &tx, &mut mgr).await);
        let ev = capture
            .find_message(Level::INFO, "UNIMPLEMENTED: onOrganizationCreation")
            .expect("decoded creation log");
        assert_eq!(ev.target, "org");
        assert!(ev.has_field("text_units", "4"), "{:?}", ev.fields);
    }

    #[tokio::test]
    async fn org_creation_rejects_a_forged_length() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        let args = [0x10, 0, 0, 0, 0x53, 0];
        assert!(dispatch(1, ORG_CREATION, &args, &tx, &mut mgr).await);
        assert!(capture
            .find_event(Level::WARN, "did not decode", "truncated")
            .is_some());
    }
}

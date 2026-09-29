use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        // PET_INVOKE_ABILITY / PET_ABILITY_TOGGLE / PET_CHANGE_STANCE (88..=90)
        // used to be stubbed here. They are routed by the outer dispatcher
        // to the pets plugin (`cimmeria-cell-pets`, #962). This module
        // no longer handles them; `social_submodule_does_not_handle_pet_methods`
        // in `dispatch.rs` is the guard.
        ORG_CREATION => {
            // `onOrganizationCreation(WSTRING aOrganizationName)` (ORG-05):
            // the pending-creation check, the D-ORG10 name rule and the
            // forward to the base.
            crate::cell::cell_methods::organization::creation::on_organization_creation(
                entity_id, args, tx, space_mgr,
            )
            .await;
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

#[cfg(test)]
mod tests {
    use tracing::Level;

    use super::*;
    use crate::test_support::{make_space_manager_with_player, LogCapture};

    /// CM 94 carries only the name (audit A-09). The router hands it to the
    /// ORG-05 creation handler, which decodes it ("SG-1" is four units) and,
    /// with no registrar offer open, refuses with 134 `(0,
    /// NO_PENDING_CREATION)` and a line. Nothing reaches the base.
    #[tokio::test]
    async fn org_creation_routes_to_the_creation_handler() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager_with_player(1);
        mgr.get_entity_mut(1).unwrap().player_id = Some(100);
        let (tx, mut rx) = mpsc::channel(8);
        let args = [4, 0, 0, 0, 0x53, 0, 0x47, 0, 0x2D, 0, 0x31, 0];
        assert!(dispatch(1, ORG_CREATION, &args, &tx, &mut mgr).await);
        let ev = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", "org.create"))
            .expect("creation outcome row");
        assert_eq!(ev.target, "org");
        assert_eq!(ev.level, Level::INFO);
        assert!(
            ev.has_field("reason", "no_pending_creation"),
            "{:?}",
            ev.fields
        );
        assert!(ev.has_field("name_units", "4"), "{:?}", ev.fields);
        let mut sent = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::EntityMethodCall {
                    method_index, args, ..
                } => sent.push((method_index, args)),
                other => panic!("nothing may reach the base: {other:?}"),
            }
        }
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0], (134, vec![0, 5]));
        assert_eq!(sent[1].0, 28);
    }

    #[tokio::test]
    async fn org_creation_rejects_a_forged_length() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager_with_player(1);
        let (tx, mut rx) = mpsc::channel(8);
        let args = [0x10, 0, 0, 0, 0x53, 0];
        assert!(dispatch(1, ORG_CREATION, &args, &tx, &mut mgr).await);
        assert!(capture
            .find_event(Level::WARN, "did not decode", "truncated")
            .is_some());
        assert!(rx.try_recv().is_err(), "a malformed call is not answered");
    }
}

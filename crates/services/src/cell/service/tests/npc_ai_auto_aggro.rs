//! `npc_ai_idle_auto_aggro` broadcast-coverage tests.
//!
//! Split out of the sibling `npc_ai` module when that file crossed the
//! 700-line cap. The state-machine, ability-selector, range, and
//! backup-waypoint tests stay in `npc_ai`; this file owns the
//! cell→base broadcast surface for the auto-aggro path
//! (`generate_threat`'s `Some(state)` return → appearance refresh,
//! but NOT `onStateFieldUpdate`).
//!
//! Reuses the `make_aggression_fixture` helper from the sibling
//! module via its `pub(super)` re-export.

use tokio::sync::mpsc;

/// Auto-aggro on a holstered player must broadcast a
/// `RefreshAppearance` so the client's `BeingAppearance` cache picks
/// up the server-side `weapon_holstered = false` flip from
/// `enter_player_combat`. Without this broadcast, server-state and
/// client-state desync: server thinks weapon is drawn, client still
/// shows holstered. The next fire passes
/// `needs_unholster_queue`'s `weapon_holstered=true` gate (because
/// the server says drawn), so the draw queue never runs — client
/// renders the fire animation with no weapon mesh attached.
///
/// Play-session symptom: post-auto-reload + OOC holster, the
/// player attacks a freshly-aggroed NPC and sees "hands in combat
/// pose but no weapon." Slot-swap recovers because slot-swap
/// broadcasts a fresh appearance. The fix here is the
/// broadcast-on-auto-aggro path in `npc_ai_idle_auto_aggro` —
/// `generate_threat` returning `Some(state)` MUST be followed by
/// `request_appearance_refresh` even though the design intent is
/// to skip the `onStateFieldUpdate` (the "ghost combat HUD" carve-
/// out the comment names).
///
/// Assertion shape: the cell→base channel sees exactly one
/// `RefreshAppearance` message addressed at the player. The
/// `onStateFieldUpdate` is intentionally NOT sent (covered by the
/// negative assertion at the end).
#[tokio::test]
async fn auto_aggro_broadcasts_appearance_refresh_on_holstered_player() {
    use crate::cell::messages::CellToBaseMsg;

    let mut mgr = super::npc_ai::make_aggression_fixture(200_010, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_010) {
        npc.aggression = 1;
    }
    // Pre-stage: player is holstered (post-OOC). The bug shape is
    // specifically "weapon_holstered flips false server-side
    // without telling the client" — to observe it, weapon must be
    // holstered going in so the flip is observable.
    if let Some(p) = mgr.get_entity_mut(1) {
        p.weapon_holstered = true;
    }

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    // Drain the cell→base channel and count appearance refresh +
    // state-field-update messages. Counting (rather than a bool)
    // pins exactly-one — a future regression that broadcasts twice
    // (e.g., once from the auto-aggro path and once from a stale
    // duplicate handler) would still trip a presence-only check.
    let mut refresh_count = 0usize;
    let mut saw_state_field_update_for_player = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::RefreshAppearance {
                entity_id: 1,
                holstered,
                ..
            } => {
                refresh_count += 1;
                assert!(
                    !holstered,
                    "RefreshAppearance payload must carry holstered=false (drawn) — \
                     enter_player_combat flipped the field to false; the broadcast \
                     must mirror the server-side value"
                );
            }
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                ..
            } if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE => {
                saw_state_field_update_for_player = true;
            }
            _ => {}
        }
    }

    assert_eq!(
        refresh_count, 1,
        "auto-aggro on holstered player must broadcast RefreshAppearance \
         exactly once — pre-fix the auto-aggro path discarded \
         `generate_threat`'s Some(state) return and the client's weapon-mesh \
         cache stayed desynced, producing the play-session 'no weapon mesh \
         on fire' bug; a count > 1 would mean a duplicate handler crept in"
    );
    assert!(
        !saw_state_field_update_for_player,
        "auto-aggro MUST NOT broadcast onStateFieldUpdate to the player — \
         the design carve-out prevents 'ghost combat HUD' (HUD lights up \
         before the player has any reason to know they've been seen). The \
         appearance refresh covers the weapon-mesh sync; the state field \
         waits for an explicit hit"
    );

    // Sanity: the cell-side state did flip. If this fails, the
    // assertion above passed for the wrong reason.
    let player = mgr.get_entity(1).unwrap();
    assert!(
        !player.weapon_holstered,
        "enter_player_combat must flip weapon_holstered=false on first-add \
         transition — broadcast or not, the server-side state is the \
         source of truth for subsequent fire-path gating"
    );
}

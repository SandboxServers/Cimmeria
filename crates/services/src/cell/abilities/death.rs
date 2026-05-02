//! Death-transition wire protocol — the ordered burst of methods sent to the
//! client when an entity's HEALTH drops to zero. Extracted from `use_ability`
//! so the protocol ordering invariants are visible in one place.
//!
//! The order on the wire is **load-bearing**:
//!
//! 1. (attacker only) `onTargetUpdate(0)` — drop the targeting reticle.
//! 2. (attacker only) `onStateFieldUpdate` with `BSF_InCombat` cleared — stops
//!    the client routing right-click on selected entities to `useAbility`.
//! 3. (NPC target only) `generate_loot_on_death` + `InteractionType` — the
//!    `InteractionType` update MUST land before the dead-state bit, otherwise
//!    the client locks in "shootable" cursor state on dead-state arrival and
//!    ignores the later flag change. Mirrors python `SGWMob.onDead()` which
//!    calls `setInteractionType` before the state field flip propagates.
//! 4. `onStateFieldUpdate` with the corpse's new state (dead bit set) — flips
//!    visuals + cursor.
//!
//! Caller is responsible for the death-side state mutations on the entity
//! itself (HEALTH=0, BSF_Dead set, AI state Dead, etc.) — this module only
//! handles outbound messages and the attacker's BSF_InCombat clear.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::loot_drop::generate_loot_on_death;
use super::messaging::send_entity_method;

/// `BSF_InCombat` bit position. Cleared on the attacker on every kill while
/// multi-mob threat tracking (#92) is unimplemented.
const BSF_IN_COMBAT: u32 = 1 << 3;

/// Apply the death-transition message sequence for a target that just died.
///
/// Ordering and side effects are described at the module level. `target_state`
/// is the corpse's already-mutated `state_field` (with `BSF_Dead` set), passed
/// in by the caller because the caller already had a mutable borrow.
pub(super) async fn apply_death_transition(
    target_eid: u32,
    attacker_id: u32,
    target_state: u32,
    attacker_is_player: bool,
    target_is_player: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // 1+2. Attacker side: clear targeting reticle + drop out of combat.
    if attacker_is_player {
        send_entity_method(
            attacker_id, crate::mercury::method_idx::ON_TARGET_UPDATE,
            0i32.to_le_bytes().to_vec(),
            tx, space_mgr,
        ).await;

        // Aggressive BSF_InCombat clear on every kill — safe for single-target
        // fights, wrong under multi-mob aggro. Issue #92 tracks the proper
        // per-player threatenedMobs set that drives this bit.
        let attacker_state = if let Some(p) = space_mgr.get_entity_mut(attacker_id) {
            let old = p.state_field;
            p.state_field &= !BSF_IN_COMBAT;
            if p.state_field != old { Some(p.state_field) } else { None }
        } else {
            None
        };
        if let Some(new_state) = attacker_state {
            tracing::debug!(
                attacker = attacker_id, target = target_eid, new_state,
                "death: clearing attacker BSF_InCombat (exit combat after kill)"
            );
            send_entity_method(
                attacker_id, crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                new_state.to_le_bytes().to_vec(),
                tx, space_mgr,
            ).await;
        }
    }

    // 3. Target side: roll loot then push interaction flags. Player targets
    //    don't loot or change interaction type.
    if !target_is_player {
        generate_loot_on_death(target_eid, space_mgr);

        let interaction_flags = space_mgr.get_entity(target_eid)
            .map_or(0i64, |e| e.interaction_type_flags);
        send_entity_method(
            target_eid, crate::mercury::method_idx::INTERACTION_TYPE,
            (interaction_flags as u64).to_le_bytes().to_vec(),
            tx, space_mgr,
        ).await;
    }

    // 4. Flip dead-state bit on the corpse — visuals + cursor change client-side.
    send_entity_method(
        target_eid, crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        target_state.to_le_bytes().to_vec(),
        tx, space_mgr,
    ).await;
}

//! `/kill` — kill an NPC via the canonical death path.

use tokio::sync::mpsc;

use cimmeria_entity::stats::HEALTH;

use super::feedback::send_gm_feedback;
use super::{CellToBaseMsg, SpaceManager};
use crate::cell::{abilities, combat};

/// `/kill [target_name]` — kill an NPC. Target is the named NPC (resolved by
/// `npc_name` / numeric id in the caller's space) or, when unnamed, the
/// caller's `current_target_id`. Players are never killable here.
///
/// Mirrors the canonical NPC kill path from `damage_apply`: zero HEALTH,
/// `combat::mark_npc_dead` (state flags + AI Dead + respawn stamp), clear
/// BSF_IN_COMBAT, then `apply_death_transition` for the ordered wire burst.
pub(super) async fn handle_kill(
    caller_entity_id: u32,
    target_name: Option<&str>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Resolve the target entity id.
    let target_eid: Option<u32> = match target_name {
        None => space_mgr
            .get_entity(caller_entity_id)
            .and_then(|e| e.current_target_id)
            .map(|id| id as u32),
        Some(name) => {
            let space_id = space_mgr.get_entity_space_id(caller_entity_id);
            let numeric_id = name.parse::<u32>().ok();
            space_mgr.all_npc_entity_ids().into_iter().find(|&nid| {
                space_mgr.get_entity_space_id(nid) == space_id
                    && (Some(nid) == numeric_id
                        || space_mgr
                            .get_entity(nid)
                            .and_then(|e| e.npc_name.as_deref())
                            .is_some_and(|n| n.eq_ignore_ascii_case(name)))
            })
        }
    };

    let Some(target_eid) = target_eid else {
        send_gm_feedback(caller_entity_id, "Kill failed: no target.", tx).await;
        return;
    };

    // Gate: NPCs only. Also flips dead bits + AI state under one mutable borrow.
    let (is_player, name_for_feedback) = match space_mgr.get_entity_mut(target_eid) {
        Some(target) if target.is_player => (true, None),
        Some(target) => {
            if let Some(stat) = target.stats.get_mut(HEALTH) {
                stat.set_current(0);
            }
            combat::mark_npc_dead(target);
            target.state_field &= !combat::BSF_IN_COMBAT;
            (false, target.npc_name.clone())
        }
        None => {
            send_gm_feedback(caller_entity_id, "Kill failed: target not found.", tx).await;
            return;
        }
    };

    if is_player {
        send_gm_feedback(caller_entity_id, "Kill refused: target is a player.", tx).await;
        return;
    }

    let target_state = space_mgr
        .get_entity(target_eid)
        .map_or(0, |e| e.state_field);

    // Wire burst: reticle clear, threat fanout, loot + interaction flags,
    // dead-state flip. Attacker = the GM (a player), target = NPC.
    abilities::apply_death_transition(
        target_eid,
        caller_entity_id,
        target_state,
        /* attacker_is_player */ true,
        /* target_is_player */ false,
        tx,
        space_mgr,
    )
    .await;

    // Mirror damage_apply: drain the corpse's threat list now that the
    // death-transition consumer has read it (it broadcasts the BSF_InCombat
    // clear to each aggroed player first).
    if let Some(corpse) = space_mgr.get_entity_mut(target_eid) {
        corpse.threat_list.clear();
    }

    let label = name_for_feedback.unwrap_or_else(|| format!("entity {target_eid}"));
    send_gm_feedback(caller_entity_id, &format!("Killed {label}."), tx).await;
}

#[cfg(test)]
mod tests {
    use super::super::tests_common::*;
    use super::super::{handle_gm_command, GmCommandIntent};
    use super::*;

    #[tokio::test]
    async fn kill_zeroes_target_health_and_marks_dead() {
        let mut mgr = mgr_with_player();
        // Spawn an NPC co-located with the player (id 1 is at [5,0,5]) and
        // select it as the caller's current target. Co-location matters: the
        // AoI tick below only makes the player a witness of the NPC if it's
        // within AoI radius, and the corpse's dead-state flip fans out via
        // that witness link.
        let mut rec = template("Drone");
        rec.x = 5.0;
        rec.y = 0.0;
        rec.z = 5.0;
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record_in_space(npc_id, &rec, mgr.get_entity_space_id(1).unwrap())
            .unwrap();
        // Give it some health to zero out.
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            if let Some(s) = npc.stats.get_mut(HEALTH) {
                s.max = 500;
                s.set_current(500);
            }
        }
        if let Some(p) = mgr.get_entity_mut(1) {
            p.current_target_id = Some(npc_id as i32);
        }
        // Populate the player's witness set so the corpse's dead-state flip
        // fans out via WitnessEntityMethod instead of dropping at the
        // empty-witness branch of send_entity_method (same harness shape as
        // abilities::death tests).
        let _ = mgr.compute_aoi_changes();

        let (tx, mut rx) = mpsc::channel(64);
        handle_gm_command(
            1,
            GmCommandIntent::Kill { target: None },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let npc = mgr.get_entity(npc_id).unwrap();
        assert_eq!(
            npc.stats.get(HEALTH).map(|s| s.cur),
            Some(0),
            "kill must zero target HEALTH"
        );
        assert_ne!(
            npc.state_field & combat::BSF_DEAD,
            0,
            "kill must set BSF_DEAD via mark_npc_dead"
        );
        // The death burst flips the corpse's state field on the wire.
        let msgs = drain(&mut rx);
        assert!(
            msgs.iter().any(|m| matches!(
                m,
                CellToBaseMsg::EntityMethodCall { entity_id, method_index, .. }
                    | CellToBaseMsg::WitnessEntityMethod { entity_id, method_index, .. }
                    if *entity_id == npc_id
                        && *method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE
            )),
            "death transition must emit onStateFieldUpdate for the corpse"
        );
        let fb = feedback_text_to(&msgs, 1).expect("kill must feed back");
        assert!(fb.starts_with("Killed"), "got: {fb}");
    }

    #[tokio::test]
    async fn kill_refuses_player_target() {
        let mut mgr = mgr_with_player();
        // Second player; select them as target.
        mgr.create_entity(2, "Castle", [1.0, 0.0, 1.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(2);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.current_target_id = Some(2);
        }
        let (tx, mut rx) = mpsc::channel(16);
        handle_gm_command(
            1,
            GmCommandIntent::Kill { target: None },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        // Player target must NOT be killed.
        let p2 = mgr.get_entity(2).unwrap();
        assert_eq!(p2.state_field & combat::BSF_DEAD, 0, "player must not die");
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("is a player"), "got: {fb}");
    }

    #[tokio::test]
    async fn kill_no_target_feeds_back_error() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Kill { target: None },
            &tx,
            &mut mgr,
            &[],
        )
        .await;
        let fb = feedback_text_to(&drain(&mut rx), 1).unwrap();
        assert!(fb.contains("no target"), "got: {fb}");
    }
}

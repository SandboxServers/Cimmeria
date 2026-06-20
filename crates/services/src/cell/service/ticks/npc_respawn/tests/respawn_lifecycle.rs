//! Respawn-tick state-lifecycle tests: the full reset to Idle, wire-packet
//! ordering, facing-direction restore, counted state-flag draining, the
//! kill→respawn→re-kill idempotency loop, and the no-op deadline cases.
//!
//! Split out of the monolithic `npc_respawn/tests.rs` (issue #529) — every
//! test body and assertion is byte-identical to the original.

use super::super::*;
use super::fixtures::{drain, make_mgr_with_dead_npc};
use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
use crate::mercury::method_idx;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::{FOCUS, HEALTH};

/// Happy path: a Dead NPC with `respawn_at` in the past gets fully
/// reset to Idle by the tick — HP restored, BSF_DEAD/MOVEMENT_LOCK
/// cleared, AI back to Idle, position snapped to spawn, threat /
/// nav state wiped, interaction-type restored to the pre-death
/// snapshot, respawn_at cleared.
#[tokio::test]
async fn ready_dead_npc_respawns_to_idle_at_spawn_position() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    let (tx, _rx) = mpsc::channel(64);

    npc_respawn_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(50).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle, "AI state must reset to Idle");
    assert_eq!(
        npc.state_field & BSF_DEAD,
        0,
        "BSF_DEAD must be cleared post-respawn"
    );
    assert_eq!(
        npc.state_field & BSF_MOVEMENT_LOCK,
        0,
        "BSF_MOVEMENT_LOCK must be cleared post-respawn"
    );
    let hp = npc.stats.get(HEALTH).unwrap();
    assert_eq!(hp.cur, hp.max, "HP must be restored to max");
    let focus = npc.stats.get(FOCUS).unwrap();
    assert_eq!(focus.cur, focus.max, "FOCUS must be restored to max");
    assert_eq!(
        npc.interaction_type_flags, npc.original_interaction_type_flags,
        "interaction_type must be restored to pre-death snapshot (drops INT_NormalLoot)"
    );
    assert_eq!(npc.position.x, 10.0, "position must snap to spawn X");
    assert_eq!(npc.position.z, 0.0, "position must snap to spawn Z");
    assert!(npc.threat_list.is_empty(), "threat_list must be wiped");
    assert!(npc.nav_path.is_empty(), "nav_path must be wiped");
    assert!(
        npc.respawn_at.is_none(),
        "respawn_at must be cleared after consumption"
    );
    // `respawn_secs` persists so a future death re-schedules.
    assert_eq!(npc.respawn_secs, Some(30));
}

/// Wire-order pin: respawn must emit EntityMoved BEFORE the
/// state-flip packets so the client teleports the corpse to spawn
/// before rendering it as alive. Within the state packets,
/// INTERACTION_TYPE precedes ON_STATE_FIELD_UPDATE precedes
/// ON_STAT_UPDATE (death-path-symmetric: the client locks in
/// cursor + pose state on the state-field flip, so interaction-type
/// must land first).
///
/// Regression shape: without the inline EntityMoved, the client
/// would see the corpse become alive at the death position for
/// ~100 ms before the next AoI tick fired EntityMoved with the
/// new position. Visible teleport-after-revive glitch.
#[tokio::test]
async fn respawn_emits_entity_moved_then_state_packets_in_load_bearing_order() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    let (tx, mut rx) = mpsc::channel(64);

    npc_respawn_tick(&tx, &mut mgr).await;

    // Tag each emitted message by kind so the ordering assertion
    // can see "EntityMoved before INTERACTION_TYPE" alongside the
    // intra-state-packet ordering.
    let msgs = drain(&mut rx);
    let tags: Vec<&str> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMoved { entity_id: 50, .. } => Some("moved"),
            CellToBaseMsg::WitnessEntityMethod {
                entity_id: 50,
                method_index,
                ..
            }
            | CellToBaseMsg::EntityMethodCall {
                entity_id: 50,
                method_index,
                ..
            } => match *method_index {
                method_idx::INTERACTION_TYPE => Some("int"),
                method_idx::ON_STATE_FIELD_UPDATE => Some("state"),
                method_idx::ON_STAT_UPDATE => Some("stat"),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let ix_moved = tags
        .iter()
        .position(|&t| t == "moved")
        .expect("respawn must emit EntityMoved");
    let ix_int = tags
        .iter()
        .position(|&t| t == "int")
        .expect("respawn must emit INTERACTION_TYPE");
    let ix_state = tags
        .iter()
        .position(|&t| t == "state")
        .expect("respawn must emit ON_STATE_FIELD_UPDATE");
    let ix_stat = tags
        .iter()
        .position(|&t| t == "stat")
        .expect("respawn must emit ON_STAT_UPDATE for HP/FOCUS reset");
    assert!(
            ix_moved < ix_int,
            "EntityMoved must precede INTERACTION_TYPE (position teleports before alive-state arrives); got {tags:?}"
        );
    assert!(
            ix_int < ix_state,
            "INTERACTION_TYPE must precede ON_STATE_FIELD_UPDATE (death-path-symmetric ordering); got {tags:?}"
        );
    assert!(
        ix_state < ix_stat,
        "ON_STATE_FIELD_UPDATE must precede ON_STAT_UPDATE; got {tags:?}"
    );
}

/// **A1 fix pin**: respawn must restore `entity.direction` from
/// the `spawn_direction` snapshot, not clobber it to (0, 0, 0).
/// Pre-fix bug shape: every respawned NPC faced north because
/// `update_entity_position` overwrote direction from the
/// `[0, 0, 0]` param. Also pins the full-precision restore — the
/// helper's `[i8; 3]` direction param would truncate 1.57 rad to
/// 2 if we passed it through, so the fix bypasses the helper for
/// direction.
#[tokio::test]
async fn respawn_restores_spawn_facing_direction() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    // Fixture spawned the NPC with yaw 1.57. Mid-fight, the AI
    // tick rotated the NPC to face its target — simulate that by
    // overwriting direction before the respawn fires.
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.direction = cimmeria_common::Vector3::new(0.0, std::f32::consts::PI, 0.0);
    }
    let (tx, _rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(50).unwrap();
    assert!(
        (npc.direction.y - 1.57).abs() < 1e-4,
        "respawn must restore spawn_direction.y exactly (1.57 rad), got {}",
        npc.direction.y,
    );
    assert_eq!(npc.direction.x, 0.0);
    assert_eq!(npc.direction.z, 0.0);
}

/// `respawn_secs = None` → `respawn_at` is never stamped at the
/// death site, so the tick scan never admits this NPC. The corpse
/// stays Dead forever. Pre-existing one-shot-mob behavior preserved.
#[tokio::test]
async fn no_respawn_when_deadline_unset() {
    let mut mgr = make_mgr_with_dead_npc(None, None);
    let (tx, mut rx) = mpsc::channel(64);

    npc_respawn_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(50).unwrap();
    assert_eq!(
        npc.ai_state,
        AiState::Dead,
        "Dead NPC without respawn_at must stay Dead"
    );
    assert_ne!(npc.state_field & BSF_DEAD, 0, "BSF_DEAD must remain set");
    assert!(
        drain(&mut rx).is_empty(),
        "no respawn → no wire messages emitted"
    );
}

/// Deadline in the future → not yet ready, tick is a no-op.
/// Mirrors the eager-promotion guard the tick uses to filter
/// candidates.
#[tokio::test]
async fn future_deadline_is_a_no_op() {
    let future = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut mgr = make_mgr_with_dead_npc(Some(60), Some(future));
    let (tx, mut rx) = mpsc::channel(64);

    npc_respawn_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(50).unwrap();
    assert_eq!(npc.ai_state, AiState::Dead, "future deadline → still Dead");
    assert!(npc.respawn_at.is_some(), "future deadline must persist");
    assert!(
        drain(&mut rx).is_empty(),
        "no wire emissions for future deadline"
    );
}

/// Fallback path: when `original_interaction_type_flags = 0` (NPC
/// spawned via bare `CellEntity::new` rather than
/// `spawn_npc_from_record`), the respawn must strip
/// `INT_NormalLoot` off the live flags instead of clobbering them
/// to 0. Other content bits the death/loot path may have OR-merged
/// in must survive.
#[tokio::test]
async fn zero_snapshot_falls_back_to_stripping_loot_bit() {
    use crate::cell::abilities::INT_NORMAL_LOOT;

    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));

    // Override the fixture: snapshot is zero, but the live flags
    // carry both a content bit (1<<5) AND the death-OR'd
    // INT_NormalLoot. Realistic for an NPC the test harness
    // synthesised without going through the record-load path.
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.original_interaction_type_flags = 0;
        npc.interaction_type_flags = (1 << 5) | INT_NORMAL_LOOT;
    }

    let (tx, _rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(50).unwrap();
    assert_eq!(
        npc.interaction_type_flags,
        1 << 5,
        "zero snapshot → must strip INT_NormalLoot while preserving other bits, not clobber to 0",
    );
}

/// `state_flag_counts` for `BSF_DEAD` and `BSF_MOVEMENT_LOCK` must
/// be drained on respawn, not just the `state_field` bits. The
/// regression this guards: after kill → respawn → kill again, the
/// second death would re-set the bits via the counted helper, and
/// if the counter was still at 1 from the first death the new
/// count would be 2 — requiring two unsets to actually clear the
/// bit. The respawn-after-second-kill would clear the bit but
/// leave the counter at 1, sticking the bits on the third death.
#[tokio::test]
async fn respawn_drains_counted_state_flag_entries() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));

    // Pre-condition: the fixture's `set_state_flag(BSF_DEAD)`
    // populated the counter. Verify before the tick so we know we
    // have something to drain.
    assert_eq!(
        mgr.get_entity(50)
            .unwrap()
            .state_flag_counts
            .get(&BSF_DEAD)
            .copied(),
        Some(1),
        "fixture must populate the BSF_DEAD counter via set_state_flag",
    );

    let (tx, _rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(50).unwrap();
    assert!(
        !npc.state_flag_counts.contains_key(&BSF_DEAD),
        "BSF_DEAD entry must be removed from state_flag_counts on respawn",
    );
    assert!(
        !npc.state_flag_counts.contains_key(&BSF_MOVEMENT_LOCK),
        "BSF_MOVEMENT_LOCK entry must be removed from state_flag_counts on respawn",
    );
}

/// Full kill → respawn → re-kill → respawn loop. Pins that the
/// respawn machinery is reentrant: `respawn_at` clears + re-stamps,
/// `state_flag_counts` doesn't accumulate, HP / position / nav /
/// movement-type state are clean on every cycle. The shape that
/// would break this is any "set once" reset (e.g., a one-time
/// flag that latched after the first respawn).
#[tokio::test]
async fn kill_respawn_rekill_loop_is_idempotent() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    let (tx, _rx) = mpsc::channel(64);

    // Cycle 1: corpse → respawn.
    npc_respawn_tick(&tx, &mut mgr).await;
    {
        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(npc.ai_state, AiState::Idle, "cycle 1 → Idle");
        assert!(npc.respawn_at.is_none(), "cycle 1 → respawn_at cleared");
    }

    // Re-kill (mimic damage_apply's kill site).
    {
        let npc = mgr.get_entity_mut(50).unwrap();
        npc.set_state_flag(BSF_DEAD);
        npc.set_state_flag(BSF_MOVEMENT_LOCK);
        npc.ai_state = AiState::Dead;
        if let Some(hp) = npc.stats.get_mut(HEALTH) {
            hp.set_current(0);
        }
        // Re-stamp the deadline.
        npc.respawn_at = Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
        // Mimic death's loot-bit OR-merge so the snapshot-restore
        // path is exercised again.
        npc.interaction_type_flags |= crate::cell::abilities::INT_NORMAL_LOOT;
        // Drop the NPC away from spawn so the position snap is
        // observable again.
        npc.position = cimmeria_common::Vector3::new(99.0, 0.0, 99.0);
    }

    // Cycle 2: respawn again.
    npc_respawn_tick(&tx, &mut mgr).await;
    {
        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(npc.ai_state, AiState::Idle, "cycle 2 → Idle");
        assert_eq!(
            npc.state_field & BSF_DEAD,
            0,
            "cycle 2 → BSF_DEAD cleared on second respawn",
        );
        assert!(
            !npc.state_flag_counts.contains_key(&BSF_DEAD),
            "cycle 2 → counter entry drained (regression guard for stick-on bug)",
        );
        assert_eq!(npc.position.x, 10.0, "cycle 2 → re-snapped to spawn");
        let hp = npc.stats.get(HEALTH).unwrap();
        assert_eq!(hp.cur, hp.max, "cycle 2 → HP restored");
        assert!(npc.respawn_at.is_none(), "cycle 2 → respawn_at cleared");
    }
}

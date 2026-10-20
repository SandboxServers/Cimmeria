//! Server-held movement lock tests (`BSF_PLAYING_MINIGAME`).
//!
//! These exercise `movement_lock::check` through the cell seam: a held
//! player may move at most 0.5 m (`HELD_SLACK_M`) horizontally from its hold
//! anchor (where the hold found it; an authorized teleport moves it), a game
//! master is not gated, and a stun's movement lock alone is left to its
//! existing paths. Reverting the gate in `client_move.rs` makes the
//! moving-2-m test accept the move and fail.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::TimedEffectSpec;
use cimmeria_entity::movement_validation::MovementReject;
use cimmeria_wire::state_field::{BSF_MOVEMENT_LOCK, BSF_PLAYING_MINIGAME};

use crate::cell::space_manager::movement_lock::HELD_SLACK_M;
use crate::cell::space_manager::ClientMoveOutcome;

use super::super::make_manager;
use super::{seed_clock, SPAWN_POS};

/// Put entity 100 at the spawn point, seed its clock, and set `state` on it.
fn held_manager(state: u32) -> (crate::cell::space_manager::SpaceManager, Instant) {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);
    mgr.get_entity_mut(100).unwrap().state_field |= state;
    (mgr, t0)
}

/// A held player standing still is accepted: the server has no reason to
/// refuse a position that did not change.
#[test]
fn held_standing_still_is_accepted() {
    let (mut mgr, t0) = held_manager(BSF_PLAYING_MINIGAME);
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        SPAWN_POS,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "a held player standing still must be accepted, got {outcome:?}"
    );
}

/// A held player inside the 0.5 m slack (float noise, the last step) is
/// accepted.
#[test]
fn held_move_within_slack_is_accepted() {
    let (mut mgr, t0) = held_manager(BSF_PLAYING_MINIGAME);
    let near = [SPAWN_POS[0] + 0.4, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        near,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "a held move of 0.4 m (inside the slack) must be accepted, got {outcome:?}"
    );
}

/// A held player walking 2 m is rejected with `MovementLocked`, snapped back
/// to `last_valid`, and the tracked position does not move.
#[test]
fn held_move_beyond_slack_is_rejected_and_position_stays() {
    let (mut mgr, t0) = held_manager(BSF_PLAYING_MINIGAME);
    let far = [SPAWN_POS[0] + 2.0, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        far,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::MovementLocked,
                last_valid,
                ..
            } if last_valid == SPAWN_POS
        ),
        "a held move of 2 m must be rejected as MovementLocked at last_valid, got {outcome:?}"
    );
    assert_eq!(
        mgr.get_entity(100).unwrap().position,
        Vector3::new(SPAWN_POS[0], SPAWN_POS[1], SPAWN_POS[2]),
        "a rejected held move must not write the proposed position"
    );
}

/// The Prison Boot (MG-63b) is a movement-only entry, and it holds the wearer
/// the same way: a booted player walking 2 m is rejected as `MovementLocked`.
/// Dropping the boot from `movement_held` makes this move pass.
#[test]
fn booted_move_beyond_slack_is_rejected() {
    let (mut mgr, t0) = held_manager(0);
    mgr.get_entity_mut(100).unwrap().apply_timed_effect(
        TimedEffectSpec {
            effect_id: 1939,
            state_flags: BSF_MOVEMENT_LOCK,
            movement_only: true,
            duration_secs: None,
            ..Default::default()
        },
        t0,
    );
    let far = [SPAWN_POS[0] + 2.0, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        far,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::MovementLocked,
                ..
            }
        ),
        "a booted move of 2 m must be rejected as MovementLocked, got {outcome:?}"
    );
}

/// Death-then-respawn (authority finding 1): the respawn's hard reset keeps the
/// boot, so a booted move of 2 m is still rejected as `MovementLocked`. Forfeiting
/// the hold on reset makes this move pass.
#[test]
fn respawned_booted_move_is_still_held() {
    let (mut mgr, t0) = held_manager(0);
    mgr.get_entity_mut(100).unwrap().apply_timed_effect(
        TimedEffectSpec {
            effect_id: 1939,
            state_flags: BSF_MOVEMENT_LOCK,
            movement_only: true,
            duration_secs: None,
            ..Default::default()
        },
        t0,
    );
    mgr.get_entity_mut(100).unwrap().clear_all_state_flags();
    let far = [SPAWN_POS[0] + 2.0, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        far,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::MovementLocked,
                ..
            }
        ),
        "a respawned booted move of 2 m must be rejected as MovementLocked, got {outcome:?}"
    );
}

/// Authorized displacement (authority finding 2): `note_authorized_teleport`
/// drops the hold anchor but not the boot, so a booted move of 2 m is still
/// rejected afterwards. Any future displacement must keep this invariant.
#[test]
fn authorized_teleport_keeps_the_boot_hold() {
    let (mut mgr, t0) = held_manager(0);
    mgr.get_entity_mut(100).unwrap().apply_timed_effect(
        TimedEffectSpec {
            effect_id: 1939,
            state_flags: BSF_MOVEMENT_LOCK,
            movement_only: true,
            duration_secs: None,
            ..Default::default()
        },
        t0,
    );
    mgr.note_authorized_teleport(100);
    let far = [SPAWN_POS[0] + 2.0, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        far,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::MovementLocked,
                ..
            }
        ),
        "a booted move of 2 m after an authorized teleport must be rejected, got {outcome:?}"
    );
}

/// A game master is not gated, even while held: the GM allowance keys on
/// `CellEntity::access_level` (2 = GameMaster, as in `gm_navmesh`).
#[test]
fn held_game_master_move_is_accepted() {
    let (mut mgr, t0) = held_manager(BSF_PLAYING_MINIGAME);
    mgr.get_entity_mut(100).unwrap().access_level = 2;
    let far = [SPAWN_POS[0] + 2.0, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        far,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "a GM must not be gated by the minigame hold, got {outcome:?}"
    );
}

/// A stun's movement lock (bit 6) without the minigame bit is not gated here:
/// a 2 m move is accepted as before.
#[test]
fn stun_lock_alone_does_not_gate_movement() {
    let (mut mgr, t0) = held_manager(BSF_MOVEMENT_LOCK);
    let far = [SPAWN_POS[0] + 2.0, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        far,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        !matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::MovementLocked,
                ..
            }
        ),
        "a stun lock without the minigame bit must not raise MovementLocked, got {outcome:?}"
    );
}

/// A modified client walking in 0.4 m steps, each inside the per-packet
/// slack, is stopped once the total walk from the hold anchor passes 0.5 m.
/// Without the anchor the steps would be accepted one by one and the player
/// would creep away.
#[test]
fn held_creeping_client_is_stopped_at_the_anchor() {
    let (mut mgr, t0) = held_manager(BSF_PLAYING_MINIGAME);
    let outcomes: Vec<ClientMoveOutcome> = (1..=8u64)
        .map(|i| {
            let x = SPAWN_POS[0] + 0.4 * i as f32;
            mgr.apply_client_position_update_at(
                t0 + Duration::from_millis(100 * i),
                100,
                [x, SPAWN_POS[1], SPAWN_POS[2]],
                [0, 0, 0],
                [0.0; 3],
            )
        })
        .collect();
    assert!(
        matches!(outcomes[0], ClientMoveOutcome::Accepted { .. }),
        "the first 0.4 m step is inside the slack, got {outcomes:?}"
    );
    let first_reject = outcomes
        .iter()
        .position(|o| {
            matches!(
                o,
                ClientMoveOutcome::Rejected {
                    reason: MovementReject::MovementLocked,
                    ..
                }
            )
        })
        .unwrap_or_else(|| panic!("the creep must be rejected at the anchor, got {outcomes:?}"));
    assert!(
        outcomes[first_reject + 1..]
            .iter()
            .all(|o| !matches!(o, ClientMoveOutcome::Accepted { .. })),
        "no step after the first reject may be accepted, got {outcomes:?}"
    );
    let pos = mgr.get_entity(100).unwrap().position;
    let dx = pos.x - SPAWN_POS[0];
    let dz = pos.z - SPAWN_POS[2];
    assert!(
        (dx * dx + dz * dz).sqrt() <= HELD_SLACK_M,
        "the held player must stay within the slack of the anchor, at x={}",
        pos.x
    );
}

/// An authorized teleport during the hold moves the anchor with the player:
/// the server put the player 5 m away, so a step from the new spot is judged
/// from there and accepted.
#[test]
fn held_anchor_moves_with_an_authorized_teleport() {
    let (mut mgr, t0) = held_manager(BSF_PLAYING_MINIGAME);
    let at_anchor = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        SPAWN_POS,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(at_anchor, ClientMoveOutcome::Accepted { .. }),
        "the held packet at the anchor must be accepted, got {at_anchor:?}"
    );
    let moved = [SPAWN_POS[0] + 5.0, SPAWN_POS[1], SPAWN_POS[2]];
    mgr.update_position_preserving_facing(100, moved, [0.0; 3]);
    mgr.note_authorized_teleport(100);
    let step = [SPAWN_POS[0] + 5.3, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(300),
        100,
        step,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "a step from the post-teleport spot must be judged from it, got {outcome:?}"
    );
}

/// A released hold leaves no stale anchor: once the bit is cleared the player
/// may move freely, and the next hold anchors where they now stand.
#[test]
fn released_hold_leaves_no_stale_anchor() {
    let (mut mgr, t0) = held_manager(BSF_PLAYING_MINIGAME);
    let at_anchor = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        SPAWN_POS,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(matches!(at_anchor, ClientMoveOutcome::Accepted { .. }));
    mgr.get_entity_mut(100).unwrap().state_field &= !BSF_PLAYING_MINIGAME;
    let free = [SPAWN_POS[0] + 2.0, SPAWN_POS[1], SPAWN_POS[2]];
    let released = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(1000),
        100,
        free,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(released, ClientMoveOutcome::Accepted { .. }),
        "an unheld player must move freely, got {released:?}"
    );
    mgr.get_entity_mut(100).unwrap().state_field |= BSF_PLAYING_MINIGAME;
    let rehold = [SPAWN_POS[0] + 2.3, SPAWN_POS[1], SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(1100),
        100,
        rehold,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "a new hold must anchor at the released spot, not the old one, got {outcome:?}"
    );
}

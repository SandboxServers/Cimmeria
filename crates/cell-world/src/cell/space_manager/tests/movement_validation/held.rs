//! Server-held movement lock tests (`BSF_PLAYING_MINIGAME`).
//!
//! These exercise `movement_lock::check` through the cell seam: a held
//! player may move at most 0.5 m (`HELD_SLACK_M`) horizontally from its last
//! valid position, a game master is not gated, and a stun's movement lock
//! alone is left to its existing paths. Reverting the gate in `client_move.rs` makes
//! the moving-2-m test accept the move and fail.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::MovementReject;
use cimmeria_wire::state_field::{BSF_MOVEMENT_LOCK, BSF_PLAYING_MINIGAME};

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

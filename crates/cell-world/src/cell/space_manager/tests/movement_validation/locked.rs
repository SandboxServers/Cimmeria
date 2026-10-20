//! Content movement hold tests (`MovementHold`, cellblock-boot-lock D-BL4).
//!
//! These drive `apply_client_position_update_at` with a hold inserted on the
//! cell entity. A held player may move at most `HOLD_RADIUS` horizontally from
//! the anchor. Anything further returns `Locked`, writes nothing and spends no
//! snap-back budget. Reverting the gate in `client_move.rs` makes every
//! `Locked` assertion here fail.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

use crate::cell::space_manager::{ClientMoveOutcome, MovementHold, SpaceManager};

use super::super::make_manager;
use super::SPAWN_POS;

/// A point well outside the 1 u slack around `SPAWN_POS`.
const FAR_POS: [f32; 3] = [15.0, 0.0, 20.0];

/// Put entity 100 at `SPAWN_POS` in the Agnos space and hold it there with
/// `message`. `space_id` of `None` uses the entity's own space; `Some(id)`
/// makes the hold belong to another space, so it is stale.
fn held_manager(message: Option<&str>, space_id: Option<u32>) -> SpaceManager {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let space_id = space_id.unwrap_or(mgr.entity_space[&100]);
    mgr.get_entity_mut(100)
        .unwrap()
        .extensions
        .insert(MovementHold {
            anchor: SPAWN_POS,
            space_id,
            chain_id: 1023,
            client_flag: false,
            message: message.map(str::to_owned),
            last_snap: None,
            last_feedback: None,
        });
    mgr
}

/// Move entity 100 to `pos` at `now` and return the `Locked` fields, or panic.
fn expect_locked(
    mgr: &mut SpaceManager,
    now: Instant,
    pos: [f32; 3],
) -> ([f32; 3], bool, Option<String>) {
    match mgr.apply_client_position_update_at(now, 100, pos, [0, 0, 0], [0.0; 3]) {
        ClientMoveOutcome::Locked {
            anchor,
            snap,
            feedback,
            ..
        } => (anchor, snap, feedback),
        other => panic!("expected Locked, got {other:?}"),
    }
}

#[test]
fn a_move_outside_the_slack_is_locked_and_not_written() {
    let mut mgr = held_manager(Some("held"), None);

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, FAR_POS, [0, 0, 0], [0.0; 3]);
    match outcome {
        ClientMoveOutcome::Locked {
            anchor,
            snap: true,
            feedback,
            ..
        } => {
            assert_eq!(anchor, SPAWN_POS);
            assert_eq!(feedback.as_deref(), Some("held"));
        }
        other => panic!("expected Locked with snap and feedback, got {other:?}"),
    }
    assert_eq!(
        mgr.get_entity(100).unwrap().position,
        Vector3::new(SPAWN_POS[0], SPAWN_POS[1], SPAWN_POS[2]),
    );
}

#[test]
fn a_move_inside_the_slack_falls_through() {
    let mut mgr = held_manager(Some("held"), None);

    let outcome = mgr.apply_client_position_update_at(
        Instant::now(),
        100,
        [10.5, 0.0, 20.3],
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "a move inside the slack must fall through, got {outcome:?}"
    );
}

#[test]
fn vertical_movement_is_not_locked() {
    let mut mgr = held_manager(Some("held"), None);

    let outcome = mgr.apply_client_position_update_at(
        Instant::now(),
        100,
        [10.0, 2.0, 20.0],
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        !matches!(outcome, ClientMoveOutcome::Locked { .. }),
        "a vertical-only move must not be locked, got {outcome:?}"
    );
}

#[test]
fn snaps_and_feedback_are_throttled() {
    let mut mgr = held_manager(Some("held"), None);
    let t0 = Instant::now();

    let (_, snap, feedback) = expect_locked(&mut mgr, t0, FAR_POS);
    assert!(snap);
    assert_eq!(feedback.as_deref(), Some("held"));

    let (_, snap, feedback) = expect_locked(&mut mgr, t0 + Duration::from_millis(100), FAR_POS);
    assert!(!snap);
    assert_eq!(feedback, None);

    let (_, snap, feedback) = expect_locked(&mut mgr, t0 + Duration::from_millis(600), FAR_POS);
    assert!(snap);
    assert_eq!(feedback, None);

    let (_, _, feedback) = expect_locked(&mut mgr, t0 + Duration::from_millis(5_100), FAR_POS);
    assert_eq!(feedback.as_deref(), Some("held"));
}

#[test]
fn no_message_means_no_feedback() {
    let mut mgr = held_manager(None, None);

    let (_, snap, feedback) = expect_locked(&mut mgr, Instant::now(), FAR_POS);
    assert!(snap);
    assert_eq!(feedback, None);
}

#[test]
fn a_hold_from_another_space_is_dropped() {
    let mut mgr = held_manager(Some("held"), Some(999));

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, FAR_POS, [0, 0, 0], [0.0; 3]);
    assert!(
        !matches!(outcome, ClientMoveOutcome::Locked { .. }),
        "a stale hold must not lock the player, got {outcome:?}"
    );
    assert!(!mgr
        .get_entity(100)
        .unwrap()
        .extensions
        .contains::<MovementHold>());
}

#[test]
fn the_gm_bypass_wins() {
    let mut mgr = held_manager(Some("held"), None);
    mgr.get_entity_mut(100).unwrap().movement_unrestricted = true;

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, FAR_POS, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "the GM bypass must accept a held move, got {outcome:?}"
    );
}

#[test]
fn a_locked_move_spends_no_snap_back_budget() {
    let mut mgr = held_manager(Some("held"), None);
    let t0 = Instant::now();
    for i in 0..10 {
        expect_locked(&mut mgr, t0 + Duration::from_millis(i * 10), FAR_POS);
    }

    mgr.get_entity_mut(100)
        .unwrap()
        .extensions
        .remove::<MovementHold>();
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(200),
        100,
        [-100_000.0, 0.0, 20.0],
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Rejected { .. }),
        "the held moves must not have used the snap-back budget, got {outcome:?}"
    );
}

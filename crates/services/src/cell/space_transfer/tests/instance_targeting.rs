//! Destination-instance resolution.
//!
//! The reason this packet exists at all: `find_or_create_space` — the
//! resolution the base's `CreateEntity` normally goes through — allocates a
//! **brand new** space for every create against an instanced world. It can
//! therefore never join somebody else's instance, which is exactly what
//! `.goto <player>` has to do. These tests pin that the exact instance a
//! caller names is the exact instance that reaches the wire, even when
//! several instances of the same world are loaded.

use super::*;

/// Stand up two live instances of the same instanced world, each holding one
/// player, and return `(space_a, space_b)` with the players' ids.
fn two_instances(mgr: &mut SpaceManager) -> (u32, u32) {
    let a = spawn_player(mgr, 10, INSTANCED, [1.0, 0.0, 1.0]);
    let b = spawn_player(mgr, 11, INSTANCED, [2.0, 0.0, 2.0]);
    assert_ne!(
        a, b,
        "an instanced world must allocate a distinct space per create — \
         if these collapse, the whole instance-targeting problem is a mirage"
    );
    (a, b)
}

/// Same world, two loaded instances: the transfer must carry the *exact*
/// instance it was given, not "an" instance of the right world.
///
/// Regression shape: drop `destination_space_id` from the `GateTravel`
/// message (or ignore it downstream) and the base falls back to by-world-name
/// resolution, which allocates a THIRD private instance — the GM arrives in
/// an empty copy of the map instead of next to the target player.
#[tokio::test]
async fn exact_instance_survives_to_the_wire_when_several_are_loaded() {
    let mut mgr = make_manager();
    let (space_a, space_b) = two_instances(&mut mgr);
    // Subject starts in a third, unrelated space.
    let origin = spawn_player(&mut mgr, 1, AGNOS, [0.0; 3]);
    assert_ne!(origin, space_a);
    assert_ne!(origin, space_b);

    let (tx, mut rx) = mpsc::channel(8);

    // Target instance B specifically — the "join this player's actual
    // instance" case P46 will drive through P44's name lookup.
    let outcome = transfer_player_to_space(
        1,
        &TransferDestination::in_instance(INSTANCED, space_b, [7.0, 8.0, 9.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect("targeting a loaded instance must be accepted");

    assert_eq!(
        outcome,
        TransferOutcome::Transferred {
            space_id: Some(space_b)
        },
        "the resolved destination must be instance B exactly"
    );

    match expect_gate_travel(&mut rx) {
        CellToBaseMsg::GateTravel {
            target_world_name,
            destination_space_id,
            ..
        } => {
            assert_eq!(target_world_name, INSTANCED);
            assert_eq!(
                destination_space_id,
                Some(space_b),
                "the wire message must name instance B ({space_b}), not instance A ({space_a}) \
                 and not a fresh instance"
            );
        }
        other => panic!("expected GateTravel, got {other:?}"),
    }
}

/// An instance id that isn't loaded must be refused *before* teardown. If it
/// slipped through, the base's create would silently degrade to a fresh
/// instance and the GM would land somewhere they never asked for, with no
/// error and no way back to the space they left.
#[tokio::test]
async fn unloaded_instance_id_is_rejected_before_any_teardown() {
    let mut mgr = make_manager();
    let (space_a, _space_b) = two_instances(&mut mgr);
    spawn_player(&mut mgr, 1, AGNOS, [0.0; 3]);
    let before = snapshot_origin(&mgr, 1);
    let (tx, mut rx) = mpsc::channel(8);

    // Derive an id that is guaranteed not to be loaded rather than guessing a
    // magic constant: one past the highest live space id.
    let stale = mgr.all_spaces().iter().map(|(id, _)| *id).max().unwrap() + 1;
    assert!(mgr.world_name_for_space(stale).is_none());

    let err = transfer_player_to_space(
        1,
        &TransferDestination::in_instance(INSTANCED, stale, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("an unloaded instance must be refused");

    assert_eq!(err, TransferRejected::InstanceNotLoaded(stale));
    assert_origin_untouched(&before, &mgr, 1, &mut rx);
    // Sanity: instance A is still live, i.e. the rejection didn't reap spaces.
    assert_eq!(mgr.world_name_for_space(space_a), Some(INSTANCED));
}

/// A loaded instance that belongs to a *different* world is a caller bug
/// (mismatched world name and space id). Refuse it rather than silently
/// trusting one of the two halves.
#[tokio::test]
async fn instance_belonging_to_another_world_is_rejected() {
    let mut mgr = make_manager();
    let castle = mgr.space_id_for_world(CASTLE).unwrap();
    spawn_player(&mut mgr, 1, AGNOS, [0.0; 3]);
    let before = snapshot_origin(&mgr, 1);
    let (tx, mut rx) = mpsc::channel(8);

    let err = transfer_player_to_space(
        1,
        // Claim the space belongs to the instanced world; it is really Castle.
        &TransferDestination::in_instance(INSTANCED, castle, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect_err("a world/instance mismatch must be refused");

    assert_eq!(
        err,
        TransferRejected::InstanceWorldMismatch {
            space_id: castle,
            requested_world: INSTANCED.to_string(),
            actual_world: CASTLE.to_string(),
        }
    );
    assert_origin_untouched(&before, &mgr, 1, &mut rx);
}

/// D15's default for `.gotolocation`: no instance selector, so pick the
/// first/default loaded instance — the lowest space id, which (ids being
/// allocated monotonically) is the oldest live instance.
///
/// Three instances are stood up and then the *oldest* is emptied, so the
/// expected answer moves from A to B. That separates the three
/// implementations that all agree on a static fixture: `.min()` (correct),
/// "whichever was created first" (a stale cache would still say A), and
/// "any `HashMap` entry" (which has no reason to say B). Re-running a static
/// lookup N times proves none of that — `HashMap` iteration order is fixed
/// for a given map within a process, so an arbitrary-entry bug returns the
/// same wrong answer every iteration.
#[tokio::test]
async fn default_instance_selection_follows_the_oldest_live_instance() {
    let mut mgr = make_manager();
    let (space_a, space_b) = two_instances(&mut mgr);
    let space_c = spawn_player(&mut mgr, 12, INSTANCED, [3.0, 0.0, 3.0]);
    assert!(
        space_a < space_b && space_b < space_c,
        "fixture assumes monotonic allocation"
    );
    spawn_player(&mut mgr, 1, AGNOS, [0.0; 3]);

    let dest = TransferDestination::in_world(INSTANCED, [1.0, 2.0, 3.0]);
    assert_eq!(
        resolve_destination_space(&dest, &mgr),
        Ok(Some(space_a)),
        "with A, B and C live the default must be A"
    );

    // Empty instance A (its only occupant leaves, so `destroy_entity` reaps
    // the space). The default must follow to B.
    mgr.destroy_entity(10);
    assert!(
        mgr.world_name_for_space(space_a).is_none(),
        "fixture must actually reap instance A"
    );
    assert_eq!(
        resolve_destination_space(&dest, &mgr),
        Ok(Some(space_b)),
        "once A is gone the default must move to B — not stay on a reaped id, \
         and not jump to an arbitrary live instance"
    );

    let (tx, mut rx) = mpsc::channel(8);
    let outcome = transfer_player_to_space(1, &dest, &tx, &mut mgr)
        .await
        .expect("default-instance transfer must be accepted");
    assert_eq!(
        outcome,
        TransferOutcome::Transferred {
            space_id: Some(space_b)
        }
    );
    match expect_gate_travel(&mut rx) {
        CellToBaseMsg::GateTravel {
            destination_space_id,
            ..
        } => assert_eq!(destination_space_id, Some(space_b)),
        other => panic!("expected GateTravel, got {other:?}"),
    }
}

/// A non-instanced world's default is its single startup space, not "the
/// lowest space id in the process" — the startup-space lookup has to win
/// over the scan.
#[tokio::test]
async fn default_instance_for_a_non_instanced_world_is_its_startup_space() {
    let mgr = make_manager();
    let castle = mgr.space_id_for_world(CASTLE).unwrap();
    let agnos = mgr.space_id_for_world(AGNOS).unwrap();
    assert!(agnos < castle, "fixture assumes Agnos allocated first");

    assert_eq!(
        resolve_destination_space(&TransferDestination::in_world(CASTLE, [0.0; 3]), &mgr),
        Ok(Some(castle)),
        "a non-instanced world must resolve to its own startup space"
    );
}

/// An instanced world with nobody in it has no loaded instance. That is not
/// an error: the destination stays `None` and the create path allocates a
/// fresh instance — which is exactly what `.gotolocation` into an empty
/// instanced world should do.
#[tokio::test]
async fn instanced_world_with_no_live_instance_defers_allocation_to_the_create_path() {
    let mut mgr = make_manager();
    spawn_player(&mut mgr, 1, AGNOS, [0.0; 3]);
    assert!(
        mgr.default_space_for_world(INSTANCED).is_none(),
        "fixture must start with no live instance of the instanced world"
    );
    let (tx, mut rx) = mpsc::channel(8);

    let outcome = transfer_player_to_space(
        1,
        &TransferDestination::in_world(INSTANCED, [4.0, 5.0, 6.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect("an empty instanced world is a valid destination");

    assert_eq!(outcome, TransferOutcome::Transferred { space_id: None });
    match expect_gate_travel(&mut rx) {
        CellToBaseMsg::GateTravel {
            target_world_name,
            destination_space_id,
            ..
        } => {
            assert_eq!(target_world_name, INSTANCED);
            assert_eq!(
                destination_space_id, None,
                "no live instance → let the create path allocate one"
            );
        }
        other => panic!("expected GateTravel, got {other:?}"),
    }
}

/// The transfer's departure must not take the destination instance down with
/// it. Leaving the last player of an instanced space destroys that space
/// (`destroy_entity`); if the subject happened to be alone in an instance of
/// the *same* world, a naive implementation could reap the instance the
/// caller is heading for.
#[tokio::test]
async fn departing_an_instance_does_not_reap_the_destination_instance() {
    let mut mgr = make_manager();
    let (space_a, space_b) = two_instances(&mut mgr);
    // Subject is alone in its own third instance of the same world.
    let origin = spawn_player(&mut mgr, 1, INSTANCED, [0.0; 3]);
    assert_ne!(origin, space_a);
    assert_ne!(origin, space_b);

    let (tx, _rx) = mpsc::channel(8);
    let outcome = transfer_player_to_space(
        1,
        &TransferDestination::in_instance(INSTANCED, space_b, [1.0, 2.0, 3.0]),
        &tx,
        &mut mgr,
    )
    .await
    .expect("transfer between instances of the same world must be accepted");

    assert_eq!(
        outcome,
        TransferOutcome::Transferred {
            space_id: Some(space_b)
        }
    );
    assert_eq!(
        mgr.world_name_for_space(space_b),
        Some(INSTANCED),
        "the destination instance must still be loaded after the subject leaves its own"
    );
    assert!(
        mgr.world_name_for_space(origin).is_none(),
        "the subject's now-empty instance is reaped, as normal"
    );
}

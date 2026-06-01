//! Cell-seam bounds-check tests for `apply_client_position_update`.
//!
//! These exercise the validator at the same seam the `EntityMove`
//! handler hits in production — `SpaceManager::apply_client_position_update`
//! — and pin the load-bearing invariants of the layer:
//!
//! - In-bounds positions land in `cell_entity.position` (Accepted).
//! - Out-of-bounds positions on X / Y / Z each fire the Rejected path,
//!   carry the last-valid position, and **do not** advance the cell
//!   entity (so the next AoI tick rebroadcasts the last-valid pos).
//! - The fallback AABB protects spaces without a loaded navmesh.
//! - **Canonical authorized-teleport regression guard**: when the
//!   server-authoritative path snaps an entity to a position outside
//!   the validator's would-be AABB via `update_entity_position`, a
//!   subsequent client position update near that new pos must NOT be
//!   rejected as out-of-bounds — i.e. the validator looks at the
//!   *current* space bounds, not a stale snapshot of where the entity
//!   used to be.
//!
//! Future PRs add speed (PR2), teleport detection + allowlist (PR3),
//! and navmesh containment (PR4) tests beside these.

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::MovementReject;

use super::super::ClientMoveOutcome;
use super::make_manager;

/// Spawn position used by every test in this module. Sits well inside
/// the Agnos space's `MinX/MaxX/MinY/MaxY` (-2400..2200, -3200..2800).
const SPAWN_POS: [f32; 3] = [10.0, 0.0, 20.0];

#[test]
fn legitimate_movement_within_bounds_accepts_and_writes() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();

    // Small delta — well inside any AABB.
    let new_pos = [12.0, 0.0, 22.0];
    let outcome = mgr.apply_client_position_update(100, new_pos, [0, 0, 0], [0.0; 3]);
    assert!(matches!(outcome, ClientMoveOutcome::Accepted { position } if position == new_pos));
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(entity.position, Vector3::new(12.0, 0.0, 22.0));
}

#[test]
fn bounds_violation_outside_x_min_rejects_and_snaps_to_last_valid() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();

    // X far below the fallback floor (-10_000). Castle Cellblock has
    // no navmesh in the test harness so we hit the FALLBACK AABB.
    let attacker_pos = [-100_000.0, 0.0, 20.0];
    let outcome = mgr.apply_client_position_update(100, attacker_pos, [0, 0, 0], [0.0; 3]);

    let (last_valid, reason) = match outcome {
        ClientMoveOutcome::Rejected {
            reason, last_valid, ..
        } => (last_valid, reason),
        other => panic!("expected Rejected(OutOfBounds), got {other:?}"),
    };
    assert_eq!(reason, MovementReject::OutOfBounds);
    assert_eq!(last_valid, SPAWN_POS);
    // Cell entity must NOT have been advanced — the next AoI tick
    // would otherwise broadcast the rejected coords to witnesses.
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(SPAWN_POS[0], SPAWN_POS[1], SPAWN_POS[2]),
        "rejected client position must not have been written to cell_entity.position"
    );
}

#[test]
fn bounds_violation_outside_y_max_rejects_and_snaps_to_last_valid() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();

    // Y above the fallback ceiling (10_000).
    let attacker_pos = [10.0, 999_999.0, 20.0];
    let outcome = mgr.apply_client_position_update(100, attacker_pos, [0, 0, 0], [0.0; 3]);

    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                last_valid,
                ..
            } if last_valid == SPAWN_POS
        ),
        "Y-max bound must reject with OutOfBounds and snap to spawn"
    );
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(entity.position, Vector3::new(10.0, 0.0, 20.0));
}

/// Floor-clip-exploit guard. A position update with valid X/Y but a Z
/// far below the floor lets a tampered client warp through terrain.
/// Z-axis omission is the documented gap; this test pins the fix.
#[test]
fn bounds_violation_outside_z_min_rejects_and_snaps_to_last_valid() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();

    let attacker_pos = [10.0, 0.0, -100_000.0];
    let outcome = mgr.apply_client_position_update(100, attacker_pos, [0, 0, 0], [0.0; 3]);

    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                last_valid,
                ..
            } if last_valid == SPAWN_POS
        ),
        "Z-min bound must reject (floor-clip exploit guard) and snap to spawn"
    );
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(entity.position, Vector3::new(10.0, 0.0, 20.0));
}

#[test]
fn rejection_carries_bounds_for_log_capture() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();

    let outcome =
        mgr.apply_client_position_update(100, [100_000.0, 0.0, 20.0], [0, 0, 0], [0.0; 3]);
    match outcome {
        ClientMoveOutcome::Rejected { bounds, .. } => {
            // The validator must surface the AABB it tested against so
            // the negative-log fields bounds_min_*/bounds_max_* carry
            // operational truth (not zeros, not unrelated defaults).
            // The fallback AABB max-X is finite-positive; pin that.
            assert!(
                bounds.max[0] > 0.0,
                "rejection bounds.max[0] must be the actual test AABB's positive ceiling"
            );
            assert!(
                bounds.min[0] < 0.0,
                "rejection bounds.min[0] must be the actual test AABB's negative floor"
            );
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

/// Regression guard: an accepted client position update must advance
/// the `last_valid` source so the *next* iteration's bounds check
/// uses the freshly written position, not the spawn pos.
///
/// The `last_valid` field returned on reject is read from
/// `space.entities[entity_id].position` at validation time. If a
/// future refactor accidentally stops writing the accepted position
/// into the cell entity (e.g. early-returning before
/// `update_entity_position`, or routing accepts down a path that
/// snapshots `last_valid` from spawn), then a later reject would
/// snap the client back to spawn instead of the actually-last-valid
/// position — surfacing as a "rubber-banding to spawn" bug for
/// players whose connection later sends one bad packet.
///
/// Shape: apply legal move A → apply legal move B → reject move C →
/// assert C's `last_valid` equals B's destination (not A, not spawn).
#[test]
fn legitimate_movement_updates_last_valid_for_next_iteration() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();

    // Move A: small legal delta from spawn.
    let pos_a = [12.0, 0.0, 22.0];
    let outcome_a = mgr.apply_client_position_update(100, pos_a, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome_a, ClientMoveOutcome::Accepted { position } if position == pos_a),
        "move A must be accepted, got {outcome_a:?}"
    );

    // Move B: another legal delta from A.
    let pos_b = [15.0, 0.0, 25.0];
    let outcome_b = mgr.apply_client_position_update(100, pos_b, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome_b, ClientMoveOutcome::Accepted { position } if position == pos_b),
        "move B must be accepted, got {outcome_b:?}"
    );

    // Move C: out-of-bounds. The rejection's `last_valid` must equal
    // pos_b — proving the cell entity's position field was advanced
    // by the prior accepts. If anything in the accept path stopped
    // writing through to `cell_entity.position`, `last_valid` would
    // be SPAWN_POS here.
    let outcome_c =
        mgr.apply_client_position_update(100, [-100_000.0, 0.0, 20.0], [0, 0, 0], [0.0; 3]);
    match outcome_c {
        ClientMoveOutcome::Rejected { last_valid, .. } => {
            assert_eq!(
                last_valid, pos_b,
                "after two accepts, snap-back target must be the most recent valid \
                 position (pos_b), not spawn or pos_a — got {last_valid:?}"
            );
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn missing_entity_returns_entity_missing_outcome() {
    let mut mgr = make_manager();
    // No create_entity — entity 9999 doesn't exist.
    let outcome = mgr.apply_client_position_update(9999, [10.0, 0.0, 20.0], [0, 0, 0], [0.0; 3]);
    assert!(matches!(outcome, ClientMoveOutcome::EntityMissing));
}

/// **Canonical failure-mode regression guard** for the movement
/// validator: an authorized server-side teleport (e.g. ring transport,
/// respawn, content-engine teleport, `handle_teleport_player`'s
/// `BASEMSG_FORCED_POSITION` snap) must NOT leave the validator in a
/// state that rejects the next legitimate client position update.
///
/// The bounds layer's specific contract: the validator looks at the
/// **current** space's AABB on every check; an authorized teleport
/// that moves the entity inside the same space's bounds (the only
/// shape PR1 catches — PR3 will extend this to the per-entity
/// authorized-teleport allowlist that protects the speed/teleport
/// layers) does not get re-rejected. Without this guard, a future
/// refactor that snapshotted the AABB at create-time and never
/// refreshed it would silently break authorized teleports inside the
/// same space.
///
/// Reverting `apply_client_position_update`'s bounds-check call to
/// the unchecked `update_entity_position` would still pass this test
/// (since it's testing the accept side); reverting the post-teleport
/// position-write would make `entity.position` stay at SPAWN_POS,
/// pushing the post-teleport client update beyond a tight radius
/// and tripping the assertion below.
#[test]
fn test_authorized_teleport_does_not_trigger_bounds_anomaly() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();

    // Step 1: server-authoritative teleport via the unchecked path
    // (this is what handle_teleport_player / ring transport /
    // respawn / content-engine teleport all do today).
    let teleport_dst = [500.0, 0.0, 500.0];
    mgr.update_entity_position(100, teleport_dst, [0, 0, 0], [0.0; 3]);

    // Step 2: client reports a position adjacent to the teleport
    // destination (the natural follow-up of a legitimate teleport —
    // the client smooths from the FORCED_POSITION snap onward).
    let client_followup = [500.1, 0.0, 500.1];
    let outcome = mgr.apply_client_position_update(100, client_followup, [0, 0, 0], [0.0; 3]);

    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Accepted { position } if position == client_followup
        ),
        "authorized teleport followed by a small-delta client update must be \
         Accepted — got {outcome:?}. A regression that uses a stale AABB or \
         rejects on the absolute distance from spawn would fire here."
    );
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(500.1, 0.0, 500.1),
        "post-teleport client update must have been written to cell_entity.position"
    );
}

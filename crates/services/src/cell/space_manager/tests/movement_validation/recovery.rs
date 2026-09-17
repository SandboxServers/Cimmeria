//! Termination of the snap-back correction loop, and the GM off-navmesh
//! allowance.
//!
//! The rest of the movement-validation suite proves that a bad client
//! position is *rejected*. This module proves the reject actually ends:
//! a correction is only useful if the position it corrects **to** is one
//! the validator would itself accept, and the server is the only party
//! that can notice when it isn't.
//!
//! # The bug shape
//!
//! Observed in production: a GM's avatar ended up at `[0, 0, 0]` in
//! Castle Cellblock — a point that is inside the space but off the
//! walkable navmesh. From then on every inbound `AVATAR_UPDATE_EXPLICIT`
//! was rejected as `OffNavmesh`, and the reject path snapped the client
//! back to `last_valid` — which *was* `[0, 0, 0]`. The client obeyed,
//! re-reported it, and was rejected again, at ~12-15 corrections/second,
//! until the player disconnected. The same shape reaches an ordinary
//! player through any server-authoritative write of an unwalkable
//! position: a stale persisted `sgw_player` row on reconnect, a content
//! teleport to authored-but-unreachable coordinates, or a GM `.gotoxyz`.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::{MovementReject, MovementValidator};
use cimmeria_entity::navigation::NavMesh;

use super::super::super::ClientMoveOutcome;
use super::{make_manager, SPAWN_POS};

/// Outside `SpaceBounds::FALLBACK`'s X floor (-10 000), which is the AABB
/// Agnos uses (no navmesh in the test harness). Standing here makes the
/// entity's own authoritative position an invalid snap-back target.
const UNREACHABLE_POS: [f32; 3] = [-50_000.0, 0.0, 20.0];

/// One step of "the client does exactly what the server told it to".
enum Step {
    Corrected([f32; 3]),
    Settled,
}

fn apply(mgr: &mut super::super::super::SpaceManager, at: Instant, pos: [f32; 3]) -> Step {
    match mgr.apply_client_position_update_at(at, 100, pos, [0, 0, 0], [0.0; 3]) {
        ClientMoveOutcome::Accepted { .. } => Step::Settled,
        ClientMoveOutcome::Rejected { last_valid, .. } => Step::Corrected(last_valid),
        ClientMoveOutcome::Recovered { recovered_to, .. } => Step::Corrected(recovered_to),
        ClientMoveOutcome::CorrectionSuppressed { .. } => Step::Settled,
        ClientMoveOutcome::EntityMissing => panic!("fixture entity vanished"),
    }
}

/// **Canonical rubber-band regression guard.**
///
/// Drives the exact production loop: the server rejects, tells the client
/// where to go, the client goes there and reports it, and round it comes
/// again. The whole sequence must reach a steady state within the
/// correction budget.
///
/// Reverting `reject_outcome` to the old unconditional
/// `ClientMoveOutcome::Rejected { last_valid, .. }` makes every iteration
/// hand the client back the same unreachable position, and the loop runs
/// until this test's iteration cap — the correction count blows the budget
/// and the assertion fires.
#[test]
fn snap_back_to_an_invalid_position_terminates_instead_of_looping() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    // A server-authoritative write puts the entity somewhere the validator
    // will not accept — the `.gotoxyz` / stale-persisted-position shape.
    mgr.update_entity_position(100, UNREACHABLE_POS, [0, 0, 0], [0.0; 3]);

    let t0 = Instant::now();
    let mut client_pos = UNREACHABLE_POS;
    let mut corrections = 0u32;
    for tick in 0..40u32 {
        match apply(
            &mut mgr,
            t0 + Duration::from_millis(100 * u64::from(tick)),
            client_pos,
        ) {
            Step::Settled => break,
            Step::Corrected(next) => {
                corrections += 1;
                client_pos = next;
            }
        }
    }

    assert!(
        corrections <= MovementValidator::MAX_SNAP_BACK_CORRECTIONS,
        "an obedient client must stop being corrected within the budget \
         ({} allowed) — got {corrections}, i.e. the server kept snapping it \
         back to a position it then rejected",
        MovementValidator::MAX_SNAP_BACK_CORRECTIONS
    );
}

/// The recovery itself: the entity is relocated cell-side (so AoI stops
/// advertising the unreachable point) and the outcome names both ends of
/// the move, so the caller can snap the owning client to the new position
/// rather than the old one.
#[test]
fn recovery_relocates_the_entity_and_reports_both_ends() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    mgr.update_entity_position(100, UNREACHABLE_POS, [0, 0, 0], [0.0; 3]);

    let outcome = mgr.apply_client_position_update_at(
        Instant::now(),
        100,
        UNREACHABLE_POS,
        [0, 0, 0],
        [0.0; 3],
    );

    let recovered_to = match outcome {
        ClientMoveOutcome::Recovered {
            reason,
            from,
            recovered_to,
            ..
        } => {
            assert_eq!(reason, MovementReject::OutOfBounds);
            assert_eq!(
                from, UNREACHABLE_POS,
                "the outcome must name where it was stuck"
            );
            recovered_to
        }
        other => panic!("expected Recovered, got {other:?}"),
    };

    // Agnos has neither a navmesh nor an authored respawner in this
    // harness, so recovery falls through to the AABB clamp.
    assert_ne!(recovered_to, UNREACHABLE_POS);
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(recovered_to[0], recovered_to[1], recovered_to[2]),
        "recovery must be written through to the cell entity — witnesses read \
         this on the next AoI tick, and a recovery that only told the client \
         would leave the server still advertising the unreachable point"
    );

    // And the loop is genuinely broken: the client's next packet, from
    // where it was just put, is ordinary traffic again.
    let settled =
        mgr.apply_client_position_update_at(Instant::now(), 100, recovered_to, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(settled, ClientMoveOutcome::Accepted { .. }),
        "the post-recovery packet must be accepted, got {settled:?}"
    );
}

/// The ordinary case is untouched: when the entity's own position is fine,
/// a bad client packet still gets the plain snap-back, and the entity is
/// **not** relocated. Recovery must be the exception, not a new default.
#[test]
fn sound_snap_back_target_still_takes_the_ordinary_reject_path() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();

    let outcome = mgr.apply_client_position_update_at(
        Instant::now(),
        100,
        UNREACHABLE_POS,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected { last_valid, .. } if last_valid == SPAWN_POS
        ),
        "a sound snap-back target must produce the ordinary correction, got {outcome:?}"
    );
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(entity.position, Vector3::new(10.0, 0.0, 20.0));
}

/// A tampered client that ignores every correction gets corrected a bounded
/// number of times and then stops being corrected at all. Its position is
/// still never written, so witnesses are unaffected — the only thing that
/// stops is the outbound `FORCED_POSITION` stream, which is what turns
/// into a rubber-band on the wire.
#[test]
fn a_client_that_ignores_corrections_stops_being_corrected() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();

    for strike in 1..=MovementValidator::MAX_SNAP_BACK_CORRECTIONS {
        let outcome = mgr.apply_client_position_update_at(
            t0 + Duration::from_millis(100 * u64::from(strike)),
            100,
            UNREACHABLE_POS,
            [0, 0, 0],
            [0.0; 3],
        );
        assert!(
            matches!(outcome, ClientMoveOutcome::Rejected { .. }),
            "strike {strike} is within budget and must still correct, got {outcome:?}"
        );
    }

    let over = mgr.apply_client_position_update_at(
        t0 + Duration::from_secs(1),
        100,
        UNREACHABLE_POS,
        [0, 0, 0],
        [0.0; 3],
    );
    match over {
        ClientMoveOutcome::CorrectionSuppressed { strikes, .. } => assert_eq!(
            strikes,
            MovementValidator::MAX_SNAP_BACK_CORRECTIONS + 1,
            "the suppressed outcome must carry the strike count for the operator log"
        ),
        other => panic!("expected CorrectionSuppressed past the budget, got {other:?}"),
    }
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(SPAWN_POS[0], SPAWN_POS[1], SPAWN_POS[2]),
        "suppressing the correction must not accept the position"
    );
}

/// The budget is per-incident, not per-session: one accepted position
/// clears it, so a player who hits a rough patch, recovers, and hits
/// another one later still gets corrected the second time.
#[test]
fn an_accepted_position_resets_the_correction_budget() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    let mut tick = 0u64;
    let mut next = || {
        tick += 1;
        t0 + Duration::from_millis(100 * tick)
    };

    for _ in 0..MovementValidator::MAX_SNAP_BACK_CORRECTIONS {
        mgr.apply_client_position_update_at(next(), 100, UNREACHABLE_POS, [0, 0, 0], [0.0; 3]);
    }
    let good =
        mgr.apply_client_position_update_at(next(), 100, [11.0, 0.0, 20.0], [0, 0, 0], [0.0; 3]);
    assert!(matches!(good, ClientMoveOutcome::Accepted { .. }));

    let outcome =
        mgr.apply_client_position_update_at(next(), 100, UNREACHABLE_POS, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Rejected { .. }),
        "the budget must reset on an accepted position, got {outcome:?}"
    );
}

// ── GM off-navmesh allowance ─────────────────────────────────────────────

/// A GM is allowed off the walkable mesh: inspecting a broken spawn or an
/// unreachable region means standing where a player cannot. The allowance
/// keys on `CellEntity::access_level`, which comes from
/// `account.accesslevel` at login and is never a client-supplied byte.
///
/// Pairs with `off_navmesh_position_is_rejected_and_not_observed` in the
/// parent module, which proves the same position **is** rejected for an
/// ordinary player — reverting the gate makes these two disagree.
#[test]
fn gm_off_navmesh_position_is_accepted_not_snapped() {
    let nav_path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav_path.exists() {
        return; // fixture-less CI — skip
    }
    let navmesh = NavMesh::load(nav_path).expect("load castle_cellblock.nav");
    let (bmin, bmax) = (navmesh.bmin, navmesh.bmax);

    let mut mgr = make_manager();
    let on_mesh = [-289.465, 68.542, -154.276];
    let space_id = mgr
        .create_entity(100, "Castle_CellBlock", on_mesh, [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    // GameMaster — the same level `cell::dispatch::gm_gate` requires.
    mgr.get_entity_mut(100).unwrap().access_level = 2;

    // Search a window around the spawn rather than the whole AABB: the
    // layer under test is navmesh containment, and a point hundreds of
    // units away would be caught by the teleport gate first (which stays
    // enforced for GMs — the allowance is navmesh-only). The ±30 u box
    // keeps every candidate inside `TELEPORT_JUMP_UNITS`, and clamping it
    // into the nav AABB keeps the bounds layer out of it too.
    const R: f32 = 30.0;
    let lo = [
        (on_mesh[0] - R).max(bmin[0]),
        bmin[1],
        (on_mesh[2] - R).max(bmin[2]),
    ];
    let hi = [
        (on_mesh[0] + R).min(bmax[0]),
        bmax[1],
        (on_mesh[2] + R).min(bmax[2]),
    ];
    let Some(off_mesh) = super::find_off_mesh_point(&mgr, 100, lo, hi, on_mesh[1]) else {
        return; // no unwalkable point nearby — nothing to assert against
    };
    assert!(
        !mgr.is_position_valid(100, &Vector3::new(off_mesh[0], off_mesh[1], off_mesh[2])),
        "scanned point must read as off-navmesh — precondition for the allowance"
    );

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, off_mesh, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == off_mesh),
        "an off-navmesh position from a GM caller must be accepted, got {outcome:?}"
    );
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(off_mesh[0], off_mesh[1], off_mesh[2]),
        "the GM's position must be written through so witnesses follow them off-mesh"
    );
}

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
//! The speed/teleport (layers 2+3) and navmesh-containment (layer 4)
//! seam tests live at the bottom of this file; they drive the time-
//! injected [`SpaceManager::apply_client_position_update_at`] so the
//! kinematics deltas are deterministic.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::MovementReject;
use cimmeria_entity::navigation::NavMesh;

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

// ── Layers 2+3: speed (warn-only) + teleport (hard reject) ───────────────

/// Seed the validator clock with a zero-delta update so the *next*
/// update has a baseline to measure against. The first update for an
/// entity always seeds-and-accepts (no prior sample), so kinematics
/// tests need this priming call.
fn seed_clock(mgr: &mut super::super::SpaceManager, entity_id: u32, now: Instant) {
    let outcome =
        mgr.apply_client_position_update_at(now, entity_id, SPAWN_POS, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "clock seed (zero-delta) must be accepted, got {outcome:?}"
    );
}

/// **Canonical teleport regression guard.** A captured
/// `AVATAR_UPDATE_EXPLICIT` whose `new_pos` is 100 units from `last_pos`
/// over 50 ms (= 2000 u/s, ~246× the 8.125 u/s class top speed) must be
/// rejected at the validation seam, and the cell entity must NOT advance
/// (so AoI never observes the teleported position).
///
/// Reverting the kinematics wiring in `apply_client_position_update_at`
/// makes this Accept and writes the 100 u jump through — the guard fires.
#[test]
fn teleport_100m_over_50ms_is_rejected_and_not_observed() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    // 100 units along +X from spawn, 50 ms later. Inside the Agnos AABB
    // (so bounds passes) and Agnos has no navmesh (so containment fails
    // open) — kinematics is the only layer that can catch it.
    let teleport = [SPAWN_POS[0] + 100.0, 0.0, SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(50),
        100,
        teleport,
        [0, 0, 0],
        [0.0; 3],
    );

    match outcome {
        ClientMoveOutcome::Rejected {
            reason, last_valid, ..
        } => {
            assert_eq!(reason, MovementReject::Teleport);
            assert_eq!(last_valid, SPAWN_POS);
        }
        other => panic!("expected Rejected(Teleport), got {other:?}"),
    }
    // AoI reads the cell entity position; it must still be spawn.
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(SPAWN_POS[0], SPAWN_POS[1], SPAWN_POS[2]),
        "teleport-rejected position must not have been written to the cell entity"
    );
}

/// Sub-teleport but over-tolerance speed (5 u in 50 ms = 100 u/s): the
/// warn-only speed layer accepts the move (and logs/counts it) — it must
/// NOT snap the client back. This pins the warn-only policy: a fast-but-
/// short hop is observed, not punished, until the tolerance is
/// calibrated from telemetry.
#[test]
fn over_tolerance_short_hop_is_accepted_warn_only() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    // 5 u (< 50 u teleport gate) in 50 ms → 100 u/s (> 1.5× top speed).
    let hop = [SPAWN_POS[0] + 5.0, 0.0, SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(50),
        100,
        hop,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == hop),
        "over-tolerance short hop must be accepted under the warn-only policy, got {outcome:?}"
    );
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(entity.position, Vector3::new(hop[0], hop[1], hop[2]));
}

/// Authorized server teleport followed by a client follow-up packet must
/// not be teleport-rejected: `note_authorized_teleport` reseeds the clock
/// and `last_pos` is the (already-written) destination, so the small
/// follow-up delta is well within tolerance. This is the speed/teleport
/// analogue of the bounds-layer authorized-teleport guard above.
#[test]
fn authorized_teleport_then_client_followup_is_accepted() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    // Server-authoritative jump far across the world (what ring/respawn/
    // gate/GM travel do) + reseed, exactly as the production paths now do.
    let dst = [900.0, 0.0, 900.0];
    mgr.update_entity_position(100, dst, [0, 0, 0], [0.0; 3]);
    mgr.note_authorized_teleport(100);

    // Client's first post-teleport packet, a small delta from dst, 100 ms
    // later. Distance from last_pos (= dst) is tiny → not a teleport.
    let followup = [900.4, 0.0, 900.2];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        followup,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == followup),
        "post-authorized-teleport follow-up must be accepted, got {outcome:?}"
    );
}

/// dt-inflation guard. The processing clock must advance on **every**
/// packet, including ones a cheaper layer rejects — otherwise an attacker
/// can spam out-of-bounds (or off-navmesh) packets to let `dt` grow, then
/// send one large jump whose implied speed (`distance / dt`) looks slow
/// enough to clear the teleport gate.
///
/// Shape: seed → bounds-rejected packet 5 s later → 100 u jump 50 ms after
/// that. The jump must still be `Teleport` (dt measured from the rejected
/// packet, ~50 ms → 2000 u/s). Reverting the up-front `touch_clock` so the
/// clock only advances inside `check_kinematics` makes `dt` ≈ 5.05 s, the
/// implied speed ≈ 20 u/s clears the gate, and the jump slips through —
/// this guard fires.
#[test]
fn bounds_reject_spam_cannot_inflate_dt_to_slip_a_teleport() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    // 5 s of "silence" spent spamming a cheaply-rejected out-of-bounds
    // packet (outside the fallback AABB). It must reject AND advance the
    // clock to this instant.
    let oob = [50_000.0, 0.0, SPAWN_POS[2]];
    let spam = mgr.apply_client_position_update_at(
        t0 + Duration::from_secs(5),
        100,
        oob,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            spam,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                ..
            }
        ),
        "spam packet must be bounds-rejected, got {spam:?}"
    );

    // 100 u jump, 50 ms after the rejected packet. dt must be measured
    // from the rejected packet (~50 ms), not the seed (~5 s).
    let jump = [SPAWN_POS[0] + 100.0, 0.0, SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_secs(5) + Duration::from_millis(50),
        100,
        jump,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::Teleport,
                ..
            }
        ),
        "jump after bounds-reject spam must still be Teleport-rejected — the \
         clock must advance on rejects so dt can't be inflated; got {outcome:?}"
    );
}

// ── Layer 4: navmesh containment ─────────────────────────────────────────

/// **Canonical off-navmesh regression guard.** A captured
/// `AVATAR_UPDATE_EXPLICIT` whose `new_pos` is inside the space AABB but
/// off the walkable navmesh must be rejected (snap-back), and the cell
/// entity must not advance.
///
/// Uses the real `castle_cellblock.nav` fixture, injected into the
/// instanced space (the production loader keys off a cwd-relative path
/// that the crate test harness doesn't satisfy). Self-skips when the
/// fixture is absent, per the repo's standard navmesh-test pattern.
#[test]
fn off_navmesh_position_is_rejected_and_not_observed() {
    let nav_path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav_path.exists() {
        return; // fixture-less CI — skip
    }
    let navmesh = NavMesh::load(nav_path).expect("load castle_cellblock.nav");
    let (bmin, bmax) = (navmesh.bmin, navmesh.bmax);

    let mut mgr = make_manager();
    // Known on-navmesh guard spawn (shared with the entity-crate nav tests).
    let on_mesh = [-289.465, 68.542, -154.276];
    let space_id = mgr
        .create_entity(100, "Castle_CellBlock", on_mesh, [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);

    assert!(
        mgr.is_position_valid(100, &Vector3::new(on_mesh[0], on_mesh[1], on_mesh[2])),
        "guard spawn must read as on-navmesh — fixture/precondition sanity"
    );

    // The nav AABB hugs the walkable polys, so corners snap to mesh
    // (DEST_EXTENTS is a 3 u box). Scan the interior XZ grid at floor
    // height for a point inside a wall / cell gap that reads off-mesh but
    // stays within the bounds AABB. The cellblock is a prison interior —
    // such points exist. Deterministic over the fixed fixture.
    let mut off_mesh: Option<[f32; 3]> = None;
    let y = on_mesh[1];
    let (mut x, step) = (bmin[0] + 2.0, 2.0_f32);
    'scan: while x < bmax[0] - 2.0 {
        let mut z = bmin[2] + 2.0;
        while z < bmax[2] - 2.0 {
            let cand = [x, y, z];
            if !mgr.is_position_valid(100, &Vector3::new(cand[0], cand[1], cand[2])) {
                off_mesh = Some(cand);
                break 'scan;
            }
            z += step;
        }
        x += step;
    }
    let off_mesh = match off_mesh {
        Some(p) => p,
        // Whole interior walkable (not expected for a cellblock) — skip
        // rather than assert, so a future re-bake can't false-fail.
        None => return,
    };
    assert!(
        !mgr.is_position_valid(100, &Vector3::new(off_mesh[0], off_mesh[1], off_mesh[2])),
        "scanned point must read as off-navmesh — precondition for the reject"
    );

    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, off_mesh, [0, 0, 0], [0.0; 3]);
    match outcome {
        ClientMoveOutcome::Rejected { reason, .. } => {
            assert_eq!(reason, MovementReject::OffNavmesh);
        }
        other => panic!("expected Rejected(OffNavmesh), got {other:?}"),
    }
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(on_mesh[0], on_mesh[1], on_mesh[2]),
        "off-navmesh position must not have been written to the cell entity"
    );
}

/// A small legitimate move that stays on the navmesh must be accepted —
/// the containment layer must not snap-fest players walking normally on a
/// navmesh-backed space. Pairs with the reject guard above.
#[test]
fn on_navmesh_small_move_is_accepted() {
    let nav_path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav_path.exists() {
        return;
    }
    let navmesh = NavMesh::load(nav_path).expect("load castle_cellblock.nav");

    let mut mgr = make_manager();
    let on_mesh = [-289.465, 68.542, -154.276];
    let space_id = mgr
        .create_entity(100, "Castle_CellBlock", on_mesh, [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);

    // A nearby on-mesh point the sibling entity-crate pathfind test uses.
    let nearby = [-280.0, 68.0, -150.0];
    if !mgr.is_position_valid(100, &Vector3::new(nearby[0], nearby[1], nearby[2])) {
        return; // geometry guard: skip if this sample isn't walkable here
    }
    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, nearby, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == nearby),
        "small on-navmesh move must be accepted, got {outcome:?}"
    );
}

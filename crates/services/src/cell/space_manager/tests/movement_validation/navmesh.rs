//! Layer 4 — navmesh containment, and the jump-height case that exercises
//! the same layer's vertical tolerance.
//!
//! These use the real `castle_cellblock.nav` fixture, injected into the
//! instanced space (the production loader keys off a cwd-relative path that
//! the crate test harness doesn't satisfy). Each self-skips when the fixture
//! is absent, per the repo's standard navmesh-test pattern.

use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::MovementReject;
use cimmeria_entity::navigation::NavMesh;

use super::super::super::ClientMoveOutcome;
use super::{find_off_mesh_point, make_manager};

/// **Canonical off-navmesh regression guard.** A captured
/// `AVATAR_UPDATE_EXPLICIT` whose `new_pos` is inside the space AABB but
/// off the walkable navmesh must be rejected (snap-back), and the cell
/// entity must not advance.
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

    let off_mesh = match find_off_mesh_point(&mgr, 100, bmin, bmax, on_mesh[1]) {
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

/// The real client jump apex — see `REAL_JUMP_APEX` in
/// `crates/entity/src/navigation/tests.rs` for the full derivation
/// (`jumpSpeed² / (2 * |gravity|)` from the values `build_world_params_args`
/// hands the client). Using the real apex here, not an arbitrary smaller
/// test value, is what actually pins the reported bug end-to-end: a jump
/// only up to a couple of units would not have exercised the multi-floor
/// disambiguation `is_point_valid` needs on the real `castle_cellblock`
/// fixture (see that function's doc comment).
const REAL_JUMP_APEX: f32 = 8.0 * 8.0 / (2.0 * 9.8);

/// End-to-end regression guard for the jump-height bug (reported as
/// "jumping snaps my facing to north / rubber-bands me backward"): a
/// client position update whose only change is an elevated Y (a jump
/// apex) over an otherwise-walkable XZ footprint must be **Accepted**,
/// not rejected as `OffNavmesh`. Pre-fix, `NavMesh::is_point_valid`
/// measured the raw 3D distance to the nearest polygon, so any jump
/// apex taller than `agent_radius * 2.0` (well under 2 units on this
/// fixture) read as off-mesh — the validator then rejected *every*
/// packet sent while airborne, snapping the player back to the
/// last-valid (pre-jump) position and, via `build_teleport_bundle`'s
/// zeroed direction, resetting their facing.
#[test]
fn jump_in_place_is_accepted_not_rejected() {
    let nav_path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav_path.exists() {
        return; // fixture-less CI — skip
    }
    let navmesh = NavMesh::load(nav_path).expect("load castle_cellblock.nav");

    let mut mgr = make_manager();
    let on_mesh = [-289.465, 68.542, -154.276];
    let space_id = mgr
        .create_entity(100, "Castle_CellBlock", on_mesh, [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);

    // Same XZ as the (validated-on-mesh) spawn, lifted to the real client
    // jump apex.
    let mid_jump = [on_mesh[0], on_mesh[1] + REAL_JUMP_APEX, on_mesh[2]];
    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, mid_jump, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == mid_jump),
        "a mid-jump position update over the same walkable footprint must be \
         accepted, not off-navmesh-rejected — got {outcome:?}"
    );
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(mid_jump[0], mid_jump[1], mid_jump[2]),
        "accepted jump position must have been written to the cell entity"
    );
}

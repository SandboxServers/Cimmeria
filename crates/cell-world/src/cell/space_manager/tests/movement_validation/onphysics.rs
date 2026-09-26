//! GM movement-validator bypass tests (onPhysics / fly-ghost).
//!
//! Split out from `movement_validation/mod.rs` (was 840 lines, over the
//! 700-line hard cap) at its own natural seam — these exercise
//! `SpaceManager::apply_client_position_update_at`'s `movement_unrestricted`
//! bypass branch specifically, not the general bounds/navmesh/speed/
//! teleport layers the parent module covers.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::MovementReject;
use cimmeria_entity::navigation::NavMesh;

use crate::cell::space_manager::ClientMoveOutcome;

use super::super::make_manager;
use super::{seed_clock, SPAWN_POS};

/// With `movement_unrestricted` set, a position that would normally fail
/// Layer 1 (bounds) must be accepted, and the entity's tracked position
/// must actually advance to the proposed value.
#[test]
fn feat_onphysics_bypass_accepts_out_of_bounds_position() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(100).unwrap().movement_unrestricted = true;

    // Same position that `bounds_violation_outside_x_min_rejects_and_snaps_to_last_valid`
    // proves gets rejected without the bypass.
    let far_out = [-100_000.0, 0.0, 20.0];
    let outcome = mgr.apply_client_position_update(100, far_out, [0, 0, 0], [0.0; 3]);

    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == far_out),
        "out-of-bounds move must be accepted under the physics bypass, got {outcome:?}"
    );
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(far_out[0], far_out[1], far_out[2]),
        "bypass must still write the tracked position, not merely report Accepted"
    );
}

/// With `movement_unrestricted` set, a position off the loaded navmesh
/// (Layer 4) must also be accepted — the bypass runs before all four
/// layers, not just the bounds layer.
#[test]
fn feat_onphysics_bypass_accepts_off_navmesh_position() {
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
        None => return,
    };

    mgr.get_entity_mut(100).unwrap().movement_unrestricted = true;
    let outcome =
        mgr.apply_client_position_update_at(Instant::now(), 100, off_mesh, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == off_mesh),
        "off-navmesh move must be accepted under the physics bypass, got {outcome:?}"
    );
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(off_mesh[0], off_mesh[1], off_mesh[2])
    );
}

/// With `movement_unrestricted` set, a teleport-shaped jump (Layers 2+3)
/// must also be accepted.
#[test]
fn feat_onphysics_bypass_accepts_teleport_shaped_jump() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);
    mgr.get_entity_mut(100).unwrap().movement_unrestricted = true;

    // Same shape as `teleport_100m_over_50ms_is_rejected_and_not_observed`.
    let teleport = [SPAWN_POS[0] + 100.0, 0.0, SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(50),
        100,
        teleport,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == teleport),
        "teleport-shaped jump must be accepted under the physics bypass, got {outcome:?}"
    );
}

/// **Negative-control regression guard.** With `movement_unrestricted`
/// left at its default `false`, all layers must reject exactly as before
/// — this is the test that proves the bypass is scoped to the flagged
/// entity and didn't accidentally disable validation globally.
#[test]
fn feat_onphysics_default_unrestricted_false_still_rejects_out_of_bounds() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    assert!(
        !mgr.get_entity(100).unwrap().movement_unrestricted,
        "movement_unrestricted must default to false"
    );

    let attacker_pos = [-100_000.0, 0.0, 20.0];
    let outcome = mgr.apply_client_position_update(100, attacker_pos, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                ..
            }
        ),
        "without the bypass flag, out-of-bounds must still reject, got {outcome:?}"
    );
    let entity = &mgr.spaces[&65536].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(SPAWN_POS[0], SPAWN_POS[1], SPAWN_POS[2])
    );
}

/// **NaN/Infinity poisoning guard.** A non-finite position sent while
/// `movement_unrestricted` is set must be rejected outright, not written
/// through, and a subsequent real teleport-shaped jump (after physics is
/// restored) must still be hard-rejected. This pins the exploit shape the
/// unconditional `is_finite()` guard closes: if a NaN ever reached
/// `cell_entity.position`, `check_kinematics`'s `distance_to` would
/// become NaN, and every `distance > TELEPORT_JUMP_UNITS` comparison
/// silently and permanently evaluates `false` under IEEE754 — disabling
/// the teleport gate for that entity until disconnect. Must fail if the
/// `is_finite()` guard is reverted.
#[test]
fn feat_onphysics_bypass_rejects_non_finite_position_and_preserves_teleport_gate() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);
    mgr.get_entity_mut(100).unwrap().movement_unrestricted = true;

    // Attempt to poison the entity's tracked position with NaN while
    // bypassed.
    let poison = [f32::NAN, 0.0, 0.0];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(10),
        100,
        poison,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                ..
            }
        ),
        "non-finite position must be rejected even under the physics bypass, got {outcome:?}"
    );
    let entity = &mgr.spaces[&65536].entities[&100];
    assert!(
        entity.position.x.is_finite()
            && entity.position.y.is_finite()
            && entity.position.z.is_finite(),
        "cell entity position must never become non-finite, got {:?}",
        entity.position
    );

    // Restore physics.
    mgr.get_entity_mut(100).unwrap().movement_unrestricted = false;

    // A real teleport-shaped jump must still be hard-rejected — proves
    // the kinematics layer's distance calculation was never poisoned.
    let teleport = [SPAWN_POS[0] + 100.0, 0.0, SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(60),
        100,
        teleport,
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
        "teleport gate must still work after an attempted NaN poison, got {outcome:?}"
    );
}

/// **Scoping regression guard.** With two entities in the same space,
/// only the one with `movement_unrestricted` set may bypass validation —
/// the other must still be rejected exactly as before. Proves the bypass
/// check reads the *calling* entity's own flag, not some shared or
/// space-level state.
#[test]
fn feat_onphysics_bypass_does_not_leak_to_other_entities_in_same_space() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(100).unwrap().movement_unrestricted = true;
    // 200 left at the default `false`.

    let far_out = [-100_000.0, 0.0, 20.0];

    let outcome_bypassed = mgr.apply_client_position_update(100, far_out, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome_bypassed, ClientMoveOutcome::Accepted { .. }),
        "flagged entity must bypass validation, got {outcome_bypassed:?}"
    );

    let outcome_normal = mgr.apply_client_position_update(200, far_out, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(
            outcome_normal,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                ..
            }
        ),
        "unflagged sibling entity must still be validated normally, got {outcome_normal:?}"
    );
    let entity_200 = &mgr.spaces[&65536].entities[&200];
    assert_eq!(
        entity_200.position,
        Vector3::new(SPAWN_POS[0], SPAWN_POS[1], SPAWN_POS[2]),
        "unflagged entity's position must not have advanced"
    );
}

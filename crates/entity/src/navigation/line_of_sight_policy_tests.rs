//! The stationary-attacker policy ([`LineOfSight::permits_stationary_attack`])
//! and the real geometry that made it necessary: the Find Ambernol drone
//! and the med-station desk on `castle_cellblock.nav` (NPC AI restoration
//! NA16, audit S11).
//!
//! Coordinates are real. Spawn 10 is the drone, template 4
//! `ArmYourself_PrisonerRetrievalUnit`, seeded `is_stationary`. Spawn 12
//! is the Ambernol vial on the desk. The player positions are the floor on
//! three sides of the desk, where a player stands to take the vial. They
//! are 13.8-16.1 m from the drone, which is the 12-16 m band where SigNoz
//! recorded `stationary_holds` with `has_los=false`.

use cimmeria_common::Vector3;

use super::{LineOfSight, NavMesh, STATIONARY_ATTACK_VERTICAL_BAND};

fn cellblock() -> Option<NavMesh> {
    let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !path.exists() {
        return None; // fixture-less checkout: skip, per the repo's navmesh-test pattern
    }
    Some(NavMesh::load(path).expect("load data/spaces/castle_cellblock.nav"))
}

/// Spawn 10, hovering about 0.85 above its floor.
const DRONE: [f32; 3] = [-220.257, 66.744, -121.375];
/// Spawn 12, the vial, on the desktop.
const VIAL: [f32; 3] = [-234.04, 66.52, -124.7];
/// The floor the drone and the player share.
const FLOOR_Y: f32 = 65.6;
/// Where a player stands to take the vial: south, west and north of the
/// desk.
const AT_THE_DESK: [[f32; 3]; 3] = [
    [-234.0, FLOOR_Y, -127.7],
    [-236.0, FLOOR_Y, -124.7],
    [-234.0, FLOOR_Y, -122.5],
];

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// The diagnosis, pinned: what the navmesh reports and why.
///
/// - The drone is **on** the mesh: its hover height is inside the jump
///   tolerance, and the projected ray starts on the floor under it. Endpoint
///   projection and eye height are not the cause.
/// - Every player position is on the mesh, on the drone's storey.
/// - The desk is a hole: the floor under the vial has no polygon, and the
///   floor either side of the desk is at the same height.
/// - So the ray from the drone to the player hits the hole's edge, and the
///   navmesh answers `Blocked` for a line a unit at eye height sees along
///   over a 1 m desk.
///
/// If a rebuilt mesh changes any of this, the test says which precondition
/// moved, rather than silently turning the regression guard below vacuous.
#[test]
fn the_med_station_desk_is_a_navmesh_hole_that_reads_blocked() {
    let Some(mesh) = cellblock() else { return };

    assert!(
        mesh.is_point_valid(&v(DRONE)),
        "precondition: the drone's spawn is on the mesh (it hovers inside the \
         jump tolerance). If this fails, S11 is an off-mesh case again, which \
         `LineOfSight::Unknown` already handles"
    );
    let ground = mesh
        .get_height_near(DRONE[0], DRONE[1], DRONE[2])
        .expect("floor under the drone");
    assert!(
        (ground - FLOOR_Y).abs() < 0.4,
        "the drone's floor moved: {ground}"
    );

    // The desk itself: no walkable surface under the vial.
    let under_vial = Vector3::new(VIAL[0], FLOOR_Y, VIAL[2]);
    assert!(
        mesh.find_nearest_poly(&under_vial)
            .is_none_or(|(_, p)| (p.x - VIAL[0]).abs() > 0.2 || (p.z - VIAL[2]).abs() > 0.2),
        "precondition: the desk under the vial is cut out of the mesh"
    );

    for p in AT_THE_DESK {
        assert!(
            mesh.is_point_valid(&v(p)),
            "player spot {p:?} must be on the mesh"
        );
        let d = ((p[0] - DRONE[0]).powi(2) + (p[2] - DRONE[2]).powi(2)).sqrt();
        assert!(
            (12.0..=16.5).contains(&d),
            "player spot {p:?} is {d} m from the drone, outside the recorded band"
        );
        assert_eq!(
            mesh.line_of_sight(&v(DRONE), &v(p)),
            LineOfSight::Blocked,
            "the navmesh ray from the drone to {p:?} crosses the desk hole and \
             reads Blocked. A different answer means the mesh changed; re-check \
             the stationary policy's regression guard in cimmeria-services"
        );
    }
}

/// The policy is what un-silences the drone: every one of those `Blocked`
/// verdicts is on the drone's storey, so a stationary attacker may fire.
/// Reverting `permits_stationary_attack` to `is_clear_or_unknown` fails
/// here.
#[test]
fn a_stationary_attacker_fires_across_the_desk() {
    let Some(mesh) = cellblock() else { return };
    for p in AT_THE_DESK {
        let los = mesh.line_of_sight(&v(DRONE), &v(p));
        let dy = p[1] - DRONE[1];
        assert!(
            los.permits_stationary_attack(dy),
            "a stationary drone must fire at {p:?} across the desk (dy {dy})"
        );
        assert!(
            !los.is_clear_or_unknown(),
            "control: the strict verdict a mobile NPC uses is still Blocked"
        );
    }
}

#[test]
fn clear_and_unknown_always_permit_a_stationary_attack() {
    for dy in [0.0, 3.9, -3.9, 50.0, -50.0, f32::NAN] {
        assert!(
            LineOfSight::Clear.permits_stationary_attack(dy),
            "Clear at dy {dy}"
        );
        assert!(
            LineOfSight::Unknown.permits_stationary_attack(dy),
            "Unknown at dy {dy}"
        );
    }
}

#[test]
fn blocked_permits_a_stationary_attack_only_inside_the_vertical_band() {
    let band = STATIONARY_ATTACK_VERTICAL_BAND;
    for dy in [0.0, 1.14, -1.14, band, -band] {
        assert!(
            LineOfSight::Blocked.permits_stationary_attack(dy),
            "same storey (dy {dy}) must fire"
        );
    }
    for dy in [band + 0.01, -(band + 0.01), 7.9, -68.0] {
        assert!(
            !LineOfSight::Blocked.permits_stationary_attack(dy),
            "another storey (dy {dy}) must hold: the band is the only floor guard"
        );
    }
    assert!(
        !LineOfSight::Blocked.permits_stationary_attack(f32::NAN),
        "a non-finite height fails closed"
    );
}

// The band stays under the tightest storey gap on the rebuilt Cellblock
// mesh (7.86 u, `tests/height.rs`), with room for a player on a ramp.
const _: () = assert!(STATIONARY_ATTACK_VERTICAL_BAND < 7.86);
const _: () = assert!(STATIONARY_ATTACK_VERTICAL_BAND >= 3.0);

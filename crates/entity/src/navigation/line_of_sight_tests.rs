//! [`NavMesh::line_of_sight`] against the shipped `harset.nav`.
//!
//! The coordinates are real Harset positions: a `spawnlist` row for the
//! meshed pair, and a player position the server rejected for the uncovered
//! one. The 2012 `harset.nav` was so fragmented that 9 of the 13 stationary
//! Harset sentries stood further than the projection box from any polygon,
//! which the strict [`NavMesh::raycast`] reports as "blocked" against every
//! possible target. The NA26 rebuild covers every one of those sentries, so
//! the uncovered endpoint below is a rooftop position a player reached that
//! the rebuilt mesh still has nothing within 3 u of.

use cimmeria_common::Vector3;

use super::{LineOfSight, NavMesh};

fn harset() -> Option<NavMesh> {
    let path = std::path::Path::new("../../data/spaces/harset.nav");
    if !path.exists() {
        return None; // fixture-less checkout: skip, per the repo's navmesh-test pattern
    }
    Some(NavMesh::load(path).expect("load data/spaces/harset.nav"))
}

/// A Harset client position `movement.validation_reject` recorded 41 times
/// (SigNoz, September 2026), and the point 8 units +Z of it. The rebuilt
/// `harset.nav` has no polygon within `DEST_EXTENTS` of either, so neither
/// can be projected.
const UNCOVERED: [f32; 3] = [-148.3, -28.3, 4.8];
const IN_FRONT_OF_UNCOVERED: [f32; 3] = [-148.3, -28.3, 12.8];

/// Spawn 224 (`FirstBug`) is ON the mesh, and so is the point 8 units +X of
/// it, yet the ray between them hits a mesh boundary. Both endpoints have
/// data, so this one is a real `Blocked`.
const SPAWN_224: [f32; 3] = [-176.697_68, -41.254, 125.271_32];
const EAST_OF_224: [f32; 3] = [-168.697_68, -41.254, 125.271_32];

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// The bug shape: an endpoint with no mesh under it is "unknown", and the
/// combat policy treats unknown as clear. Reverting `line_of_sight` to the
/// strict raycast turns this `Unknown` into a `Blocked`, which is what kept
/// 9 of the 13 Harset sentries from ever firing on the 2012 mesh.
#[test]
fn a_sentry_the_mesh_does_not_cover_is_unknown_not_blocked() {
    let Some(mesh) = harset() else { return };

    // Control: the mesh loaded and answers normally where it has data. Ring
    // region 4's pad is a coordinate the seed already puts players on.
    let pad = Vector3::new(-25.641, -67.828, 15.249);
    assert!(
        mesh.is_point_valid(&pad),
        "ring pad 4 must be on-mesh, otherwise the mesh did not load and every \
         verdict below is vacuous"
    );
    assert_eq!(
        mesh.line_of_sight(&pad, &Vector3::new(-25.2, -67.828, 15.5)),
        LineOfSight::Clear,
        "two points half a unit apart on one pad must be Clear"
    );

    assert!(
        !mesh.is_point_valid(&v(UNCOVERED)) && mesh.find_nearest_poly(&v(UNCOVERED)).is_none(),
        "precondition: the rejected rooftop position has no polygon within the \
         projection box of harset.nav. If a regenerated mesh now covers it, move \
         this test to a coordinate that is still uncovered"
    );
    let los = mesh.line_of_sight(&v(UNCOVERED), &v(IN_FRONT_OF_UNCOVERED));
    assert_eq!(
        los,
        LineOfSight::Unknown,
        "an endpoint outside mesh coverage must read Unknown, never Blocked"
    );
    assert!(
        los.is_clear_or_unknown(),
        "combat policy: unknown counts as clear"
    );
    assert!(
        !mesh.raycast(&v(UNCOVERED), &v(IN_FRONT_OF_UNCOVERED)),
        "the strict raycast keeps its old contract (off-mesh start = false); \
         callers that need the distinction use line_of_sight"
    );
}

/// The other half: `Unknown` must not swallow a real obstruction. With both
/// endpoints on the mesh a boundary hit is still `Blocked`, so a mobile NPC
/// keeps holding fire behind a wall. Since NA16 a stationary attacker fires
/// through a same-storey `Blocked` (see `line_of_sight_policy_tests`). This
/// verdict is what aggro and mobile NPCs still act on.
#[test]
fn a_boundary_between_two_meshed_points_is_still_blocked() {
    let Some(mesh) = harset() else { return };
    assert!(mesh.is_point_valid(&v(SPAWN_224)) && mesh.is_point_valid(&v(EAST_OF_224)));

    let los = mesh.line_of_sight(&v(SPAWN_224), &v(EAST_OF_224));
    assert_eq!(los, LineOfSight::Blocked);
    assert!(!los.is_clear_or_unknown());
}

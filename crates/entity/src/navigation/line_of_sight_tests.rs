//! [`NavMesh::line_of_sight`] against the shipped `harset.nav`.
//!
//! The coordinates are real `spawnlist` rows. `harset.nav` is fragmented:
//! most of the stationary Harset sentries stand further than the projection
//! box from any polygon, which the strict [`NavMesh::raycast`] reports as
//! "blocked" against every possible target.

use cimmeria_common::Vector3;

use super::{LineOfSight, NavMesh};

fn harset() -> Option<NavMesh> {
    let path = std::path::Path::new("../../data/spaces/harset.nav");
    if !path.exists() {
        return None; // fixture-less checkout: skip, per the repo's navmesh-test pattern
    }
    Some(NavMesh::load(path).expect("load data/spaces/harset.nav"))
}

/// Spawn 232 (template 159, a stationary gate sentry) and a player standing
/// 8 units in front of it (heading 0 = +Z).
const SENTRY_232: [f32; 3] = [4.695_996, -58.654_087, -188.246_61];
const IN_FRONT_OF_232: [f32; 3] = [4.695_996, -58.654_087, -180.246_61];

/// Spawn 225 (template 160) is ON the mesh, and so is the point 8 units in
/// front of it (heading 1.546 rad, roughly +X), yet the ray between them hits
/// a mesh boundary. Both endpoints have data, so this one is a real
/// `Blocked`.
const SENTRY_225: [f32; 3] = [-18.7497, -68.9228, 19.3561];
const IN_FRONT_OF_225: [f32; 3] = [-10.7521, -68.9228, 19.5532];

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// The bug shape: an endpoint with no mesh under it is "unknown", and the
/// combat policy treats unknown as clear. Reverting `line_of_sight` to the
/// strict raycast turns this `Unknown` into a `Blocked`, which is what kept
/// 9 of the 13 Harset sentries from ever firing.
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
        !mesh.is_point_valid(&v(SENTRY_232)),
        "precondition: spawn 232 is off harset.nav. If a regenerated mesh now \
         covers it, move this test to a coordinate that is still uncovered"
    );
    let los = mesh.line_of_sight(&v(SENTRY_232), &v(IN_FRONT_OF_232));
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
        !mesh.raycast(&v(SENTRY_232), &v(IN_FRONT_OF_232)),
        "the strict raycast keeps its old contract (off-mesh start = false); \
         callers that need the distinction use line_of_sight"
    );
}

/// The other half: `Unknown` must not swallow a real obstruction. With both
/// endpoints on the mesh a boundary hit is still `Blocked`, so a mobile NPC
/// keeps holding fire behind a wall. Spawn 225 is itself stationary, and
/// since NA16 a stationary attacker fires through a same-storey `Blocked`
/// (see `line_of_sight_policy_tests`). This verdict is what aggro and
/// mobile NPCs still act on.
#[test]
fn a_boundary_between_two_meshed_points_is_still_blocked() {
    let Some(mesh) = harset() else { return };
    assert!(mesh.is_point_valid(&v(SENTRY_225)) && mesh.is_point_valid(&v(IN_FRONT_OF_225)));

    let los = mesh.line_of_sight(&v(SENTRY_225), &v(IN_FRONT_OF_225));
    assert_eq!(los, LineOfSight::Blocked);
    assert!(!los.is_clear_or_unknown());
}

//! Shared navmesh helpers for the Beta E1 seed guards (UB-02b onward). They
//! are copied from the Dakara guards (`dakara_e1/travel.rs`): the shipped
//! navmesh decides standability and component reachability, and a missing
//! mesh is a test failure, never a silent skip.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

/// The Beta E1 arrival point (`ARRIVAL` in the UB-00b placement seal).
pub(super) const ARRIVAL: [f32; 3] = [635.761, -60.134, 388.448];

/// Loads a navmesh from `data/spaces`.
pub(super) fn mesh(name: &str) -> NavMesh {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces")
        .join(name);
    NavMesh::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn vector(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

fn reaches(mesh: &NavMesh, from: [f32; 3], to: [f32; 3]) -> bool {
    let path = mesh.find_path(&vector(from), &vector(to));
    matches!(path.status, PathStatus::Ok)
        && path
            .waypoints
            .last()
            .is_some_and(|end| end.distance_to(&vector(to)) < 2.0)
}

/// On the shipped navmesh, with no claim about which component it is on.
pub(super) fn standable(mesh: &NavMesh, point: [f32; 3], label: &str) {
    assert!(
        mesh.is_point_valid(&vector(point)),
        "{label} is off the shipped navmesh"
    );
}

/// On the shipped navmesh and walkable to `anchor` on the same component.
///
/// This is the one-leg check from `point` back to `anchor`. A walk is not
/// symmetric on this mesh, so a hub anchor uses [`standable_and_walkable`]
/// (forward from `ARRIVAL`) instead; the Asgard and Nine Peaks guards still
/// use this one.
#[allow(dead_code)]
pub(super) fn standable_and_connected(
    mesh: &NavMesh,
    point: [f32; 3],
    anchor: [f32; 3],
    label: &str,
) {
    standable(mesh, point, label);
    assert!(
        reaches(mesh, point, anchor),
        "{label} is on a different navmesh component than {anchor:?}"
    );
}

/// True when each leg of `from -> hops[0] -> .. -> to` is one `find_path`
/// that ends `Ok` within 2.0 m of its target. Every leg starts at the
/// authored hop, never at the previous leg's last waypoint: a `Partial` result
/// ends at the polygon nearest the goal, and restarting from it is a greedy
/// walk that stalls in a local minimum (placement seal, UB-00b-3). A chain that
/// stalls is not evidence of an island; a `false` here means a hop is missing
/// or the target really is unreachable.
pub(super) fn walkable_via(
    mesh: &NavMesh,
    from: [f32; 3],
    hops: &[[f32; 3]],
    to: [f32; 3],
) -> bool {
    let mut leg_start = from;
    for &leg_end in hops.iter().chain(std::iter::once(&to)) {
        if !reaches(mesh, leg_start, leg_end) {
            return false;
        }
        leg_start = leg_end;
    }
    true
}

/// On the shipped navmesh and walkable from `from` through `hops` (see
/// [`walkable_via`]); an empty `hops` is one direct leg.
pub(super) fn standable_and_walkable(
    mesh: &NavMesh,
    point: [f32; 3],
    from: [f32; 3],
    hops: &[[f32; 3]],
    label: &str,
) {
    standable(mesh, point, label);
    assert!(
        walkable_via(mesh, from, hops, point),
        "{label} is not walkable from {from:?} via {hops:?}"
    );
}

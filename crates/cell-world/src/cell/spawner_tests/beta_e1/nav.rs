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

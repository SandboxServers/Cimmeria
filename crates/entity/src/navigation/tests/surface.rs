//! Guards for [`NavMesh::move_along_surface`].

use super::super::NavMesh;
use super::{guard_spawn, FIXTURE};
use cimmeria_common::Vector3;

fn fixture() -> Option<NavMesh> {
    let path = std::path::Path::new(FIXTURE);
    path.exists()
        .then(|| NavMesh::load(path).expect("Failed to load castle_cellblock.nav"))
}

/// The result stands on the start's floor whatever height the caller asked
/// for: Detour's `moveAlongSurface` does not project its result, so a raw
/// copy of it would keep the request's Y (here 10 u under the floor).
#[test]
fn move_along_surface_ends_on_the_floor_of_the_start_storey() {
    let Some(mesh) = fixture() else { return };
    // Two points on the open floor the guards chase across, on the straight
    // path Detour returns from the guard spawn toward the ramp.
    let from = Vector3::new(-292.62, 68.6, -161.18);
    let to = Vector3::new(-295.94, 68.6, -165.54);
    let floor = mesh.get_height_near(from.x, from.y, from.z).unwrap();

    let end = mesh
        .move_along_surface(&from, &Vector3::new(to.x, to.y - 10.0, to.z))
        .expect("the start is on the mesh");

    let end_floor = mesh.get_height_near(end.x, floor, end.z).unwrap();
    assert!(
        (end.y - end_floor).abs() < 0.05,
        "result {end:?} must stand on the floor at {end_floor}"
    );
    assert!(
        (end.x - to.x).abs() < 0.1 && (end.z - to.z).abs() < 0.1,
        "an unobstructed move reaches its XZ target, got {end:?}"
    );
}

/// A move toward a point outside the mesh stops at the boundary instead of
/// leaving it, somewhere the pathfinder can start from. Detour's raw result
/// here lies exactly on the mesh's +X edge (-253.0), where the start-poly
/// lookup finds nothing within ±0.5; returned as-is, it fails both
/// assertions below.
#[test]
fn move_along_surface_stops_at_the_mesh_boundary() {
    let Some(mesh) = fixture() else { return };
    let g = guard_spawn();
    let far = Vector3::new(mesh.bmax[0] + 500.0, g.y, g.z);

    let end = mesh.move_along_surface(&g, &far).expect("on the mesh");

    assert!(
        end.x < mesh.bmax[0],
        "the move must stop on the mesh: {end:?}"
    );
    assert!(
        mesh.is_point_valid(&end),
        "the stopping point {end:?} must be on the mesh: {:?}",
        mesh.diagnose_point(&end)
    );
    assert!(
        mesh.find_path(&end, &g).is_some(),
        "an NPC left at {end:?} must be able to route again"
    );
}

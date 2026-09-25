//! `NavMesh::find_path`'s typed outcome against the real rebuilt
//! `castle_cellblock.nav` (1,658 polys, 17 components).

use cimmeria_common::Vector3;

use crate::navigation::{NavMesh, PathStatus};

fn cellblock() -> Option<NavMesh> {
    let p = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    p.exists()
        .then(|| NavMesh::load(p).expect("castle_cellblock.nav"))
}

/// `MessHall_Guard1`'s spawn, on the main interior island.
const MESSHALL: Vector3 = Vector3 {
    x: -96.25,
    y: 34.591,
    z: -91.59,
};

/// Two points on one island: a full corridor.
#[test]
fn a_route_within_one_island_is_ok() {
    let Some(mesh) = cellblock() else { return };
    let o = mesh.find_path(&MESSHALL, &Vector3::new(-128.853, 39.552, -73.534));
    assert_eq!(o.status, PathStatus::Ok, "{o:?}");
    assert!(o.waypoints.len() >= 2);
    assert!(o.start_snap.is_some() && o.end_snap.is_some());
}

/// The ground-plane corner is a different island: Detour's best guess is
/// returned (and still usable as a path) but flagged partial — the old FFI
/// checked only `DT_FAILURE` and reported it as a success.
#[test]
fn a_route_across_two_islands_is_partial_and_still_returned() {
    let Some(mesh) = cellblock() else { return };
    let goal = Vector3::new(-400.0, 0.2, -400.0);
    let o = mesh.find_path(&MESSHALL, &goal);
    assert_eq!(o.status, PathStatus::Partial, "{o:?}");
    let end = *o.waypoints.last().unwrap();
    assert!(
        end.distance_to(&goal) > 50.0,
        "a partial path stops at its own island's edge, far short: {end:?}"
    );
    assert!(
        o.into_waypoints().is_some(),
        "callers still receive the partial path"
    );
}

/// Hovering 2 units over the floor fails the ±0.5 start box.
#[test]
fn a_hovering_start_is_no_start_poly() {
    let Some(mesh) = cellblock() else { return };
    let hover = Vector3::new(MESSHALL.x, MESSHALL.y + 2.0, MESSHALL.z);
    let o = mesh.find_path(&hover, &MESSHALL);
    assert_eq!(o.status, PathStatus::NoStartPoly);
    assert!(o.start_snap.is_none() && o.waypoints.is_empty());
    assert!(mesh.start_poly_snap(&hover).is_none());
    assert!(mesh.start_poly_snap(&MESSHALL).is_some());
}

/// A destination nowhere near any polygon.
#[test]
fn an_unmeshed_destination_is_no_end_poly() {
    let Some(mesh) = cellblock() else { return };
    let o = mesh.find_path(&MESSHALL, &Vector3::new(-96.25, 300.0, -91.59));
    assert_eq!(o.status, PathStatus::NoEndPoly);
    assert!(o.start_snap.is_some() && o.end_snap.is_none());
}

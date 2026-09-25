//! `NavMesh::find_path`'s typed outcome against the real rebuilt
//! `castle_cellblock.nav` (1,658 polys, 17 components).

use cimmeria_common::Vector3;

use crate::navigation::{NavMesh, PathStatus};

/// Tracked in git and loaded in CI: a missing file fails, never skips.
fn cellblock() -> NavMesh {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces/castle_cellblock.nav");
    NavMesh::load(&p).unwrap_or_else(|e| panic!("load {}: {e}", p.display()))
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
    let mesh = cellblock();
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
    let mesh = cellblock();
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
    let mesh = cellblock();
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
    let mesh = cellblock();
    let o = mesh.find_path(&MESSHALL, &Vector3::new(-96.25, 300.0, -91.59));
    assert_eq!(o.status, PathStatus::NoEndPoly);
    assert!(o.start_snap.is_some() && o.end_snap.is_none());
}

/// `nearest_point_within` puts a hovering mover back on its floor, and holds
/// the answer to the requested reach: Detour's closest point may lie outside
/// the search box. Revert-proof: dropping the vertical hold returns the
/// hallway floor 4.8 u above a point beside the mess hall wall.
#[test]
fn nearest_point_within_recovers_a_hover_and_respects_its_reach() {
    let mesh = cellblock();
    let hover = Vector3::new(MESSHALL.x, MESSHALL.y + 2.0, MESSHALL.z);
    assert!(
        mesh.start_poly_snap(&hover).is_none(),
        "fails the start box"
    );
    let onto = mesh
        .nearest_point_within(&hover, 2.0, 4.0)
        .expect("the floor is 2 u below");
    assert!((onto.y - 34.6).abs() < 0.1, "{onto:?}");
    assert!(
        mesh.start_poly_snap(&onto).is_some(),
        "a path can start there"
    );

    assert!(mesh
        .nearest_point_within(
            &Vector3::new(MESSHALL.x, MESSHALL.y + 300.0, MESSHALL.z),
            2.0,
            4.0
        )
        .is_none());
    let beside_wall = Vector3::new(-126.25, 34.6, -104.59);
    assert!(mesh.nearest_point_within(&beside_wall, 8.0, 4.0).is_none());
    assert!(mesh.nearest_point_within(&beside_wall, 8.0, 5.0).is_some());
}

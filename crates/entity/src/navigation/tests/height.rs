//! Guards for [`NavMesh::get_height_near`], the storey-aware ground query.
//!
//! The guard spawn column on `castle_cellblock` has two walkable storeys:
//! the guard's floor at ~68.6 and a lower floor at ~0.2 directly beneath.
//! The pre-NA01 query searched from world Y = 0 with a ±500 box and so
//! returned ~0.2 for an NPC standing at 68.5 (audit M4 / D1).
//!
//! A 2-unit grid sweep of the whole mesh (September 2026 build) found the
//! smallest vertical gap between stacked storeys at ~7.9 units, near
//! (-194, -156). That is the column where the `±JUMP_HEIGHT_TOLERANCE`
//! search box comes closest to reaching the wrong floor, so it is pinned
//! here alongside the guard column.

use super::super::NavMesh;
use super::{guard_spawn, FIXTURE};

/// Load the fixture, or `None` when it isn't checked out (CI).
fn fixture() -> Option<NavMesh> {
    let path = std::path::Path::new(FIXTURE);
    if !path.exists() {
        return None;
    }
    Some(NavMesh::load(path).expect("Failed to load castle_cellblock.nav"))
}

/// A rebuild moving a pinned floor by more than this fails loudly rather
/// than silently re-baselining.
const TOLERANCE: f32 = 0.25;

/// Surface Y of the guard's storey, and of the storey under it.
const GUARD_UPPER_FLOOR_Y: f32 = 68.6;
const GUARD_LOWER_FLOOR_Y: f32 = 0.2;

/// The tightest stacked column on the mesh and its two floors.
const TIGHT_X: f32 = -194.0;
const TIGHT_Z: f32 = -156.0;
const TIGHT_LOWER_FLOOR_Y: f32 = 55.54;
const TIGHT_UPPER_FLOOR_Y: f32 = 63.40;

fn assert_reads(mesh: &NavMesh, x: f32, y_ref: f32, z: f32, want: f32) {
    let got = mesh
        .get_height_near(x, y_ref, z)
        .unwrap_or_else(|| panic!("({x}, {z}) from y_ref {y_ref}: no surface, want ~{want}"));
    assert!(
        (got - want).abs() < TOLERANCE,
        "({x}, {z}) from y_ref {y_ref} must read the floor at ~{want}, got {got}"
    );
}

/// The M4 bug shape: an entity on the upper storey of a stacked column
/// must read its own floor, not the one beneath. Reverting
/// `get_height_near` to the old origin-centred query returns the lower
/// floor for the upper `y_ref` and fails the first assertion.
#[test]
fn get_height_near_returns_the_storey_under_the_reference() {
    let Some(mesh) = fixture() else { return };
    let g = guard_spawn();

    // The guard's own spawn Y (68.542), and just above it.
    assert_reads(&mesh, g.x, g.y, g.z, GUARD_UPPER_FLOOR_Y);
    assert_reads(&mesh, g.x, 70.0, g.z, GUARD_UPPER_FLOOR_Y);
    assert_reads(&mesh, g.x, 0.5, g.z, GUARD_LOWER_FLOOR_Y);
}

/// The narrowest storey gap on the mesh: a reference a little above
/// either floor still reads that floor.
#[test]
fn get_height_near_separates_the_tightest_stacked_storeys() {
    let Some(mesh) = fixture() else { return };

    for floor in [TIGHT_LOWER_FLOOR_Y, TIGHT_UPPER_FLOOR_Y] {
        for above in [0.1, 3.0] {
            assert_reads(&mesh, TIGHT_X, floor + above, TIGHT_Z, floor);
        }
    }
}

/// A reference Y that is not near any surface in the column must read
/// `None`, not fall through to some other storey: callers treat `None` as
/// "no ground near this Y".
#[test]
fn get_height_near_far_from_any_surface_is_none() {
    let Some(mesh) = fixture() else { return };
    let g = guard_spawn();

    // Midway between the guard column's two storeys, ~34 units from each.
    assert_eq!(mesh.get_height_near(g.x, 34.4, g.z), None);
    // Far above the top storey.
    assert_eq!(
        mesh.get_height_near(g.x, GUARD_UPPER_FLOOR_Y + 20.0, g.z),
        None
    );
    // Just past the jump tolerance above the guard's floor.
    assert_eq!(
        mesh.get_height_near(g.x, GUARD_UPPER_FLOOR_Y + 4.5, g.z),
        None
    );
}

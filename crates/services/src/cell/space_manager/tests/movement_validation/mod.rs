//! Cell-seam tests for `apply_client_position_update`.
//!
//! These exercise the validator at the same seam the `EntityMove`
//! handler hits in production — `SpaceManager::apply_client_position_update`
//! — and pin the load-bearing invariants of each layer. This file holds only
//! the fixtures every sibling shares; the tests themselves are grouped by the
//! layer they exercise:
//!
//! - [`bounds`] — layer 1 (bounds / AABB, the Z floor-clip gate) and the
//!   `last_valid` snap-back target it feeds, including the canonical
//!   authorized-teleport guard.
//! - [`kinematics`] — layers 2+3 (speed warn-only, teleport hard reject),
//!   driven through the time-injected
//!   [`SpaceManager::apply_client_position_update_at`] so the deltas are
//!   deterministic.
//! - [`navmesh`] — layer 4 (navmesh containment) plus the jump-height case.
//! - [`advisory`] — the per-world `navmesh_mode` switch that decides whether
//!   layer 4 gates at all, against the real `harset.nav`.
//! - [`gm_navmesh`] — the GM off-navmesh allowance layered on top of layer 4.
//! - [`onphysics`] — the `movement_unrestricted` (fly/ghost) bypass.
//! - [`recovery`] — snap-back termination: relocation, correction budget,
//!   and the terminal-fallback resolver.

use std::time::Instant;

use cimmeria_common::Vector3;

use super::super::ClientMoveOutcome;
use super::make_manager;

/// Spawn position used by the Agnos-based tests across this module. Sits
/// well inside the Agnos space's `MinX/MaxX/MinY/MaxY`
/// (-2400..2200, -3200..2800).
const SPAWN_POS: [f32; 3] = [10.0, 0.0, 20.0];

/// Find a point inside the navmesh AABB that reads as **off** the walkable
/// mesh, for the tests that need one.
///
/// The nav AABB hugs the walkable polys, so corners snap to mesh
/// (`DEST_EXTENTS` is a 3 u box). Scan the interior XZ grid at `y` for a
/// point inside a wall / cell gap that reads off-mesh but stays within the
/// bounds AABB. The cellblock is a prison interior — such points exist.
/// Deterministic over the fixed fixture. `None` means the whole scanned
/// interior was walkable; callers skip rather than assert, so a future
/// re-bake cannot false-fail them.
fn find_off_mesh_point(
    mgr: &super::super::SpaceManager,
    entity_id: u32,
    bmin: [f32; 3],
    bmax: [f32; 3],
    y: f32,
) -> Option<[f32; 3]> {
    let (mut x, step) = (bmin[0] + 2.0, 2.0_f32);
    while x < bmax[0] - 2.0 {
        let mut z = bmin[2] + 2.0;
        while z < bmax[2] - 2.0 {
            if !mgr.is_position_valid(entity_id, &Vector3::new(x, y, z)) {
                return Some([x, y, z]);
            }
            z += step;
        }
        x += step;
    }
    None
}

/// Seed the validator clock with a zero-delta update so the *next*
/// update has a baseline to measure against. The first update for an
/// entity always seeds-and-accepts (no prior sample), so kinematics
/// tests need this priming call.
fn seed_clock(mgr: &mut super::super::SpaceManager, entity_id: u32, now: Instant) {
    let outcome =
        mgr.apply_client_position_update_at(now, entity_id, SPAWN_POS, [0, 0, 0], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { .. }),
        "clock seed (zero-delta) must be accepted, got {outcome:?}"
    );
}

mod advisory;
mod bounds;
mod gm_navmesh;
mod kinematics;
mod navmesh;
mod onphysics;
mod recovery;

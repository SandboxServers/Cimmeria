//! Guards for [`NavMesh::diagnose_point`] — the containment *diagnosis*
//! that turns an undifferentiated `reason = "navmesh"` reject into a
//! named gate plus distances.
//!
//! # Why these assertions avoid mesh statistics
//!
//! `data/spaces/castle_cellblock.nav` is rebuilt periodically (most
//! recently September 2026, from 1,479 to ~1,658 polys). Every assertion
//! below is therefore anchored on a **geometric fact** the sibling
//! `tests/mod.rs` guards already depend on — the guard spawn point is
//! walkable, a point 50 units under it is not, `agent_radius` is the
//! containment scale — and never on a poly/vertex count or an absolute
//! hash. The one exception is the fingerprint test, which asserts
//! internal consistency (hash length, short-hash prefix, non-zero
//! counts) rather than specific values.

use super::super::{NavGate, NavMesh, JUMP_HEIGHT_TOLERANCE};
use super::{guard_spawn, FIXTURE};
use cimmeria_common::Vector3;

/// Load the fixture, or `None` when it isn't checked out (CI).
fn fixture() -> Option<NavMesh> {
    let path = std::path::Path::new(FIXTURE);
    if !path.exists() {
        return None;
    }
    Some(NavMesh::load(path).expect("Failed to load castle_cellblock.nav"))
}

macro_rules! mesh_or_skip {
    () => {
        match fixture() {
            Some(m) => m,
            None => return,
        }
    };
}

/// A point on the walkable surface is valid, reports no gate, and still
/// carries its distances — the accepted-position sampler reports `dy`
/// as height above the surface, so the `valid` path must populate it.
#[test]
fn on_mesh_point_is_valid_with_no_gate_and_measured_distances() {
    let mesh = mesh_or_skip!();
    let v = mesh.diagnose_point(&guard_spawn());
    assert!(v.valid, "guard spawn must be on the navmesh: {v:?}");
    assert!(v.gate.is_none(), "an accepted point names no gate: {v:?}");
    assert!(
        v.horizontal_dist.is_some() && v.dy.is_some(),
        "an accepted point must still report its distances — the position \
         sampler logs `dy` as height above the walkable surface: {v:?}"
    );
    let h = v.horizontal_dist.unwrap();
    assert!(
        h < mesh.agent_radius * 2.0,
        "an accepted point is inside the horizontal gate by definition; \
         got {h} against a gate of {}",
        mesh.agent_radius * 2.0
    );
}

/// The real client jump apex (`jumpSpeed² / 2|gravity|` ≈ 3.27 units) is
/// accepted — the same fact `jump_above_navmesh_same_xz_is_still_valid`
/// pins for the boolean, asserted here through the diagnosis so a gate
/// misclassification that still returned `true` would be caught.
#[test]
fn real_jump_apex_is_valid_and_reports_positive_dy() {
    let mesh = mesh_or_skip!();
    let apex = 8.0 * 8.0 / (2.0 * 9.8);
    let g = guard_spawn();
    let v = mesh.diagnose_point(&Vector3::new(g.x, g.y + apex, g.z));
    assert!(v.valid, "the real jump apex must stay accepted: {v:?}");
    assert!(
        v.dy.is_some_and(|dy| dy > 1.0),
        "a mid-jump point must report a positive dy above the surface: {v:?}"
    );
}

/// Just past the upward tolerance but still inside the search box:
/// `above_jump_tolerance`, with a `dy` that names how far past.
///
/// 4.5 is the same height `just_above_jump_tolerance_is_still_invalid`
/// uses — between `JUMP_HEIGHT_TOLERANCE` (4.0) and the phase-2 search
/// box's vertical half-extent (5.0), so the lookup succeeds and the
/// height comparison is what actually rejects.
#[test]
fn just_above_the_jump_tolerance_reports_above_jump_tolerance() {
    let mesh = mesh_or_skip!();
    let g = guard_spawn();
    let v = mesh.diagnose_point(&Vector3::new(g.x, g.y + 4.5, g.z));
    assert!(!v.valid, "4.5 units up is past the tolerance: {v:?}");
    assert_eq!(
        v.gate,
        Some(NavGate::AboveJumpTolerance),
        "a point above the surface must be diagnosed as a height failure, \
         not as a mesh hole — telling those two apart is the whole point \
         of the gate field: {v:?}"
    );
    assert!(
        v.dy.is_some_and(|dy| dy > JUMP_HEIGHT_TOLERANCE),
        "the reported dy must exceed the tolerance it was rejected by: {v:?}"
    );
}

/// Clipped below the floor: `below_surface`, with a negative `dy`.
///
/// This is also the phase-fallback guard. At this depth the phase-2
/// search box (re-centred `JUMP_HEIGHT_TOLERANCE` *below* the query
/// point) no longer reaches the floor, so phase 2 finds nothing — and
/// without the `phase2.or(phase1)` fallback in `diagnose_point` the
/// verdict would come back `no_poly_in_extents`, mislabelling a
/// floor-clip as a mesh hole.
#[test]
fn clipped_below_the_surface_reports_below_surface_not_a_mesh_hole() {
    let mesh = mesh_or_skip!();
    let below_gate = mesh.agent_radius * 2.0;
    let g = guard_spawn();
    let v = mesh.diagnose_point(&Vector3::new(g.x, g.y - (below_gate + 0.3), g.z));
    assert!(!v.valid, "a floor-clip must still be rejected: {v:?}");
    assert_eq!(
        v.gate,
        Some(NavGate::BelowSurface),
        "under-terrain clipping must be diagnosed as below_surface. \
         `no_poly_in_extents` here would send an operator to rebuild a \
         mesh that is fine: {v:?}"
    );
    assert!(
        v.dy.is_some_and(|dy| dy < 0.0),
        "a below-surface point must report a negative dy: {v:?}"
    );
}

/// Far from any polygon: `no_poly_in_extents`, and no distances (there
/// was nothing to measure against). This is the mesh-hole signature the
/// September 2026 Castle rebuild was chasing.
#[test]
fn far_from_the_mesh_reports_no_poly_in_extents_with_no_distances() {
    let mesh = mesh_or_skip!();
    let g = guard_spawn();
    let v = mesh.diagnose_point(&Vector3::new(g.x, g.y - 50.0, g.z));
    assert!(!v.valid);
    assert_eq!(v.gate, Some(NavGate::NoPolyInExtents), "{v:?}");
    assert!(
        v.horizontal_dist.is_none() && v.dy.is_none(),
        "with no polygon found there is nothing to measure against, so the \
         distances must be absent rather than zero — a 0.0 here reads in a \
         SigNoz query as 'right on the surface': {v:?}"
    );
}

/// Every gate label is a distinct, stable, lowercase token. The
/// `movement_validation_rejects_total{gate}` label and the SigNoz saved
/// view both pin these strings.
#[test]
fn gate_labels_are_distinct_stable_tokens() {
    let all = [
        NavGate::NoPolyInExtents,
        NavGate::Horizontal,
        NavGate::BelowSurface,
        NavGate::AboveJumpTolerance,
    ];
    let labels: Vec<&str> = all.iter().map(|g| g.label()).collect();
    assert_eq!(
        labels,
        vec![
            "no_poly_in_extents",
            "horizontal",
            "below_surface",
            "above_jump_tolerance"
        ]
    );
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), labels.len(), "gate labels must be distinct");
}

/// **The wrapper invariant.** Sweeps a 3D grid spanning on-mesh,
/// off-mesh-horizontally, above and below, and asserts for every point
/// that `is_point_valid` and `diagnose_point().valid` agree, that a gate
/// is present exactly when the point is invalid, and that distances are
/// present exactly when a polygon was found.
///
/// Today `is_point_valid` delegates, so the first assertion holds by
/// construction — it is a guard against the *future* shape where someone
/// re-implements the boolean for speed and the logs start reporting a
/// gate for a point the validator accepted (or vice versa), which would
/// be invisible in production.
///
/// The gate/distance invariants are not vacuous: they fail today if
/// `verdict_against` populates `horizontal_dist` on the no-poly path or
/// `classify_containment` returns a gate for a contained point.
#[test]
fn is_point_valid_agrees_with_diagnose_point_across_a_sweep() {
    let mesh = mesh_or_skip!();
    let g = guard_spawn();

    let mut checked = 0u32;
    let mut valid_seen = 0u32;
    let mut gates_seen = std::collections::HashSet::new();

    // ±12 units horizontally in 0.5-unit steps at four heights. The
    // horizontal span deliberately crosses the walkable surface's edges:
    // the horizontal gate is agent_radius * 2 (~1.2 u) and the phase-1
    // search box reaches 3 u, so the ~1.8-unit-wide shell just outside
    // every mesh boundary at a given height is where `horizontal` fires.
    let mut x = -12.0f32;
    while x <= 12.0 {
        let mut z = -12.0f32;
        while z <= 12.0 {
            for dy in [-2.0f32, -0.25, 0.0, 3.0] {
                let p = Vector3::new(g.x + x, g.y + dy, g.z + z);
                let v = mesh.diagnose_point(&p);
                assert_eq!(
                    mesh.is_point_valid(&p),
                    v.valid,
                    "is_point_valid and diagnose_point disagreed at {p:?} — \
                     the validator would snap a player back for a reason the \
                     logs deny, or accept one the logs flag. Verdict: {v:?}"
                );
                assert_eq!(
                    v.gate.is_none(),
                    v.valid,
                    "a gate must be reported exactly when the point is \
                     invalid, at {p:?}: {v:?}"
                );
                match v.gate {
                    Some(NavGate::NoPolyInExtents) => assert!(
                        v.horizontal_dist.is_none() && v.dy.is_none(),
                        "no polygon found -> no distances, at {p:?}: {v:?}"
                    ),
                    _ => assert!(
                        v.horizontal_dist.is_some() && v.dy.is_some(),
                        "a polygon was found -> both distances present, at \
                         {p:?}: {v:?}"
                    ),
                }
                if let Some(gate) = v.gate {
                    gates_seen.insert(gate.label());
                } else {
                    valid_seen += 1;
                }
                checked += 1;
            }
            z += 0.5;
        }
        x += 0.5;
    }

    assert!(
        checked > 1000,
        "sweep must actually cover ground: {checked}"
    );
    assert!(
        valid_seen > 0,
        "a sweep centred on a known-walkable point must contain accepted \
         points, or the fixture moved out from under this test"
    );
    // `horizontal` is the classification arm no single-point test above
    // exercises, because finding a coordinate in the 1.2-3.0 u shell
    // around a mesh edge requires knowing the geometry. The sweep walks
    // that shell many times over; if this stops holding, the rebuilt
    // mesh no longer has an edge within 12 units of the guard spawn and
    // the sweep bounds need widening (not the assertion deleting).
    assert!(
        gates_seen.contains("horizontal"),
        "the sweep must cross a mesh edge so the horizontal gate is \
         actually exercised; gates seen: {gates_seen:?}"
    );
}

/// The fingerprint is populated from the real file and is
/// self-consistent. Asserts shape, not values, so a mesh rebuild does
/// not require editing this test.
#[test]
fn fingerprint_identifies_the_loaded_file() {
    let mesh = mesh_or_skip!();
    let fp = mesh.fingerprint();
    assert_eq!(fp.content_hash.len(), 16, "u64 as fixed-width hex");
    assert!(
        fp.content_hash.chars().all(|c| c.is_ascii_hexdigit()),
        "hash must be plain hex so it pastes into a log filter: {}",
        fp.content_hash
    );
    assert_eq!(mesh.short_hash(), &fp.content_hash[..8]);
    assert_eq!(
        fp.file_bytes,
        std::fs::metadata(FIXTURE).unwrap().len(),
        "file_bytes must be the real on-disk size"
    );
    assert!(fp.npolys > 0 && fp.nverts > 0);
    assert_eq!(fp.npolys, mesh.poly_count());
    assert!(
        fp.agent_height > 0.0 && fp.agent_radius > 0.0 && fp.agent_climb > 0.0,
        "agent params come from the .nav header: {fp:?}"
    );
    assert_eq!(
        fp.agent_radius, mesh.agent_radius,
        "the fingerprint's agent_radius must be the same one the \
         containment gates are derived from, or the log explains a \
         rejection with a tolerance that wasn't used"
    );
    assert!(fp.path.contains("castle_cellblock"));
}

/// Loading the same file twice must produce the same hash. Without
/// this, the hash could not be used to answer "is this the same mesh
/// the session two days ago ran on?".
#[test]
fn the_same_file_hashes_the_same_way_twice() {
    let a = mesh_or_skip!();
    let b = mesh_or_skip!();
    assert_eq!(a.fingerprint().content_hash, b.fingerprint().content_hash);
}

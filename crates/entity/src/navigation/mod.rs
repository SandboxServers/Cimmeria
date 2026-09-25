//! Server-side navigation mesh loading and queries via Detour FFI.
//!
//! Loads XRC-format `.nav` files (custom Recast output from the Cimmeria
//! NavBuilder), converts them into Detour navmesh tiles, and delegates all
//! pathfinding and spatial queries to the real Detour C++ library.
//!
//! The XRC format stores a single-tile Recast polygon mesh with detail
//! triangulation. We parse the binary format, pass the raw arrays through
//! `dtCreateNavMeshData` via our C wrapper, then init a `dtNavMesh` and
//! `dtNavMeshQuery` for runtime queries.
//!
//! Reference: `src/cellapp/entity/navigation.cpp` (C++ server implementation)
//! Reference: `tools/SceneEditor/src/commands/navmesh.rs` (XRC parser)
//!
//! Module layout:
//!
//! - [`xrc`] — XRC binary-reader helpers and the header sanity caps.
//! - [`load`] — `NavMesh::load`: parse + Detour tile construction.
//! - [`fingerprint`] — which mesh this is ([`NavMeshFingerprint`]).
//! - [`verdict`] — why a containment test said what it said
//!   ([`PointVerdict`], [`NavGate`]).
//! - [`line_of_sight`] — the line-of-sight raycast and its three-state
//!   result.
//! - [`path`] — `find_path` and its typed [`PathOutcome`] (which Detour
//!   stage failed, whether the corridor was partial, how far each end
//!   snapped).
//! - this module — the [`NavMesh`] handle and the rest of its query API.

mod fingerprint;
mod line_of_sight;
mod load;
mod path;
mod verdict;
mod xrc;

pub use line_of_sight::{LineOfSight, LosProbe};
pub use path::{PathOutcome, PathStatus};

use std::ffi::c_void;

use cimmeria_common::Vector3;

use crate::detour_ffi::{self, dt_status_failed};

pub use fingerprint::NavMeshFingerprint;
pub use verdict::{NavGate, PointVerdict};

// ── Maximum path sizes (matching C++ reference) ─────────────────────────

const MAX_POLY_PATH: i32 = 256;
const MAX_STRAIGHT_PATH: i32 = 256;

// ── Search extents (matching C++ NavigationQueryParams) ─────────────────

/// Tight extents for start position — entity should be standing on a poly.
const START_EXTENTS: [f32; 3] = [0.5, 0.5, 0.5];
/// Loose extents for destination — entity might be jumping, on a rail, etc.
const DEST_EXTENTS: [f32; 3] = [3.0, 3.0, 3.0];
/// Search box for [`NavMesh::get_height_near`], centred on the caller's
/// reference Y.
///
/// The Y half-extent reuses [`JUMP_HEIGHT_TOLERANCE`] so "near" means the
/// same thing here as in [`NavMesh::is_point_valid`]: a mover more than a
/// jump apex away from every surface is not standing on any of them. It
/// must stay well below the smallest storey gap on a shipped mesh, or a
/// query from one floor can read the floor above or below it; the
/// storey-height guards in `tests/height.rs` pin that on
/// `castle_cellblock`. The XZ half-extent is tight on purpose: the caller
/// wants the surface *under* `(x, z)`, not the nearest one beside it.
const HEIGHT_NEAR_EXTENTS: [f32; 3] = [0.5, JUMP_HEIGHT_TOLERANCE, 0.5];
/// Upward vertical containment tolerance for [`NavMesh::is_point_valid`] —
/// how far *above* the walkable surface a proposed position may sit and
/// still be accepted as "mid-jump" rather than off-mesh.
///
/// Sized from the client's own jump physics, not guessed: the server hands
/// the client `gravity = -9.8` and `jumpSpeed = 8.0` in
/// `build_world_params_args` (`crates/services/src/mercury/world_data/mod.rs`),
/// giving a ballistic apex of `jumpSpeed² / (2 * |gravity|) ≈ 3.27` world
/// units above takeoff. `4.0` adds ~0.7 units of margin for uneven ground,
/// slope, and query jitter — comfortably above the real apex instead of
/// (as an earlier version of this fix used) reusing `DEST_EXTENTS`'s `3.0`,
/// which sat *below* the apex and would still have rejected a full jump.
const JUMP_HEIGHT_TOLERANCE: f32 = 4.0;

/// Downward vertical containment tolerance for [`NavMesh::is_point_valid`].
/// Deliberately much tighter than [`JUMP_HEIGHT_TOLERANCE`] — jumping is a
/// legitimate reason to be *above* the surface, but there is no legitimate
/// reason to be *below* it. Reuses the horizontal `agent_radius * 2.0` gate
/// so under-terrain clipping stays exactly as strict as it was before the
/// jump-height fix (an earlier version of this fix used one symmetric
/// `.abs()` tolerance for both directions, which would have widened the
/// floor-clip allowance from `agent_radius * 2.0` to `4.0`).
const BELOW_SURFACE_TOLERANCE_FACTOR: f32 = 2.0;

/// Search extents used by [`NavMesh::is_point_valid`]'s below-biased retry
/// (see that method's doc comment for why a second search exists at all).
/// The vertical half-extent must comfortably exceed
/// [`JUMP_HEIGHT_TOLERANCE`] so the retry's search box — centered *below*
/// the query point — still reaches back up to it. Kept separate from
/// `DEST_EXTENTS` so this fix doesn't change search behavior for
/// path/raycast callers.
const JUMP_SEARCH_EXTENTS: [f32; 3] = [
    DEST_EXTENTS[0],
    JUMP_HEIGHT_TOLERANCE + 1.0,
    DEST_EXTENTS[2],
];

/// A loaded navigation mesh backed by the Detour C++ library.
///
/// Provides pathfinding, line-of-sight raycasting, and point validation
/// queries. Loaded from XRC-format `.nav` files produced by NavBuilder.
pub struct NavMesh {
    /// Opaque Detour query handle (wraps dtNavMeshQuery + dtQueryFilter).
    query: *mut c_void,
    /// Opaque Detour navmesh handle (dtNavMesh*).
    mesh: *mut c_void,
    /// Human-readable label (space name).
    name: String,
    /// Which `.nav` file this is: content hash, size, header counts and
    /// agent parameters. Every navmesh-related log line that names a
    /// mesh names it through this — see [`Self::short_hash`].
    fingerprint: NavMeshFingerprint,
    /// Agent configuration from the navmesh file.
    pub agent_height: f32,
    pub agent_radius: f32,
    /// World-space bounds.
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
}

// NavMesh pointers are heap-allocated C++ objects with no thread-local state.
// dtNavMeshQuery methods are const (read-only) after init, and each space
// owns its navmesh exclusively — no concurrent mutation.
unsafe impl Send for NavMesh {}
unsafe impl Sync for NavMesh {}

impl Drop for NavMesh {
    fn drop(&mut self) {
        unsafe {
            if !self.query.is_null() {
                detour_ffi::detour_free_query(self.query);
            }
            if !self.mesh.is_null() {
                detour_ffi::detour_free_navmesh(self.mesh);
            }
        }
    }
}

impl NavMesh {
    // ── Public query API ─────────────────────────────────────────────────

    /// Number of polygons in the navmesh.
    pub fn poly_count(&self) -> u32 {
        self.fingerprint.npolys
    }

    /// Which `.nav` file this mesh was built from — content hash, size,
    /// header counts, agent parameters. Logged in full once, at space
    /// creation.
    pub fn fingerprint(&self) -> &NavMeshFingerprint {
        &self.fingerprint
    }

    /// The 8-hex-digit form of the content hash, for per-event logs.
    ///
    /// Any log line that reports a navmesh decision should carry this, so
    /// "was this player walking on the rebuilt mesh or the 2013 one?" is a
    /// filter rather than a deploy-timestamp reconstruction.
    pub fn short_hash(&self) -> &str {
        &self.fingerprint.short_hash
    }

    /// Find the nearest polygon to a point.
    /// Returns (polygon_ref_as_usize, closest_point_on_poly) or None.
    pub fn find_nearest_poly(&self, pos: &Vector3) -> Option<(usize, Vector3)> {
        self.find_nearest_poly_with_extents(pos, &DEST_EXTENTS)
    }

    /// Shared FFI core of [`Self::find_nearest_poly`], parameterized on the
    /// search extents so callers that need a taller (or shorter) search box
    /// — e.g. [`Self::is_point_valid`]'s jump-aware lookup — don't have to
    /// go through `DEST_EXTENTS` and affect path/raycast callers too.
    fn find_nearest_poly_with_extents(
        &self,
        pos: &Vector3,
        extents: &[f32; 3],
    ) -> Option<(usize, Vector3)> {
        let center = [pos.x, pos.y, pos.z];
        let mut nearest_ref: u32 = 0;
        let mut nearest_pt = [0.0f32; 3];

        let status = unsafe {
            detour_ffi::detour_find_nearest_poly(
                self.query,
                center.as_ptr(),
                extents.as_ptr(),
                &mut nearest_ref,
                nearest_pt.as_mut_ptr(),
            )
        };

        if dt_status_failed(status) || nearest_ref == 0 {
            return None;
        }

        Some((
            nearest_ref as usize,
            Vector3::new(nearest_pt[0], nearest_pt[1], nearest_pt[2]),
        ))
    }

    /// Returns `true` if the given position lies on a walkable navmesh polygon.
    ///
    /// Horizontal (X/Z) containment uses a tight `agent_radius`-based gate,
    /// unchanged from before this fix. Vertical (Y) containment is
    /// deliberately asymmetric:
    ///
    /// - **Upward** tolerance is [`JUMP_HEIGHT_TOLERANCE`] (4.0 units,
    ///   comfortably above the ~3.27-unit apex the client's own jump
    ///   physics produces — see that constant's doc comment for the
    ///   derivation). A legitimately jumping avatar sits well above the
    ///   walkable surface beneath it for the length of the jump arc, and
    ///   this is client-authoritative physics the server never simulates.
    ///   The prior implementation used a single combined 3D distance check
    ///   against `agent_radius * 2` (~1.2 units on the `castle_cellblock`
    ///   fixture) for *both* directions, so any jump apex taller than that
    ///   already read as off-navmesh and triggered a snap-back on every
    ///   jump.
    /// - **Downward** tolerance stays at the original tight
    ///   `agent_radius * 2` gate ([`BELOW_SURFACE_TOLERANCE_FACTOR`]) —
    ///   there's no legitimate reason to be *below* the walkable surface,
    ///   so under-terrain clipping must stay exactly as strict as it was
    ///   pre-fix. A single symmetric tolerance here would have widened the
    ///   floor-clip allowance to match the (much larger) jump tolerance.
    ///
    /// ## Why this is a two-phase search, not one wider one
    ///
    /// Detour's `dtFindNearestPoly` returns whichever polygon is nearest to
    /// the query point in raw 3D Euclidean distance — not "the polygon
    /// directly below." Simply widening the search box's vertical extent to
    /// cover a full jump apex (an earlier version of this fix did exactly
    /// that) works for open ground but breaks on multi-level geometry: on
    /// the real `castle_cellblock` fixture, a mezzanine walkway sits ~4
    /// units above (and a few units over from) the guard-spawn floor. At a
    /// jump apex near that walkway's height, it is *closer in a straight
    /// line* to the airborne query point than the true floor is straight
    /// down, so a single widened search returns the walkway's polygon —
    /// which then fails the horizontal gate and produces exactly the
    /// false-reject this fix exists to remove.
    ///
    /// Phase 1 repeats the original, unmodified `DEST_EXTENTS`-anchored
    /// search (so ground-level movement and modest jumps are completely
    /// unaffected by this fix — same polygon, same result, as before).
    /// Phase 2 only runs when phase 1 fails, and re-centers the search
    /// `JUMP_HEIGHT_TOLERANCE` units *below* the query point — i.e. where
    /// the ground would be if the caller is at the very top of a jump —
    /// so a true floor straight down outweighs a walkway merely diagonally
    /// nearby, without needing any caller context (last-known height,
    /// grounded state, etc.) that isn't already in `pos` itself.
    pub fn is_point_valid(&self, pos: &Vector3) -> bool {
        // Thin wrapper by construction: the boolean the movement
        // validator acts on and the diagnosis the logs report are the
        // same evaluation, so they cannot drift into disagreeing about
        // whether a point is on the mesh. Reverting this to a second,
        // parallel implementation is the specific regression
        // `is_point_valid_agrees_with_diagnose_point_across_a_sweep`
        // exists to catch.
        self.diagnose_point(pos).valid
    }

    /// [`Self::is_point_valid`]'s decision **plus why**, and how far off.
    ///
    /// Same two-phase search, same gates, same answer in
    /// [`PointVerdict::valid`]. The extra information is what makes a
    /// `movement.validation_reject` row actionable: `gate` separates "the
    /// mesh has a hole here" ([`NavGate::NoPolyInExtents`]) from "the
    /// player is a metre inside the floor" ([`NavGate::BelowSurface`])
    /// from "this jump was one unit too high"
    /// ([`NavGate::AboveJumpTolerance`]), and the distances say by how
    /// much — which is the difference between "rebuild the mesh" and
    /// "widen the tolerance".
    ///
    /// ## Which phase's polygon is reported
    ///
    /// Phase 2 exists because Detour returns the polygon nearest in raw
    /// 3D distance, which on multi-level geometry can be a mezzanine
    /// rather than the floor below (see [`Self::is_point_valid`]'s doc).
    /// A valid verdict from either phase wins (phase 1 first), exactly as
    /// in the boolean check. When **both** phases fail, the verdict
    /// reports whichever polygon sits most directly above or below the
    /// query point — the smaller horizontal distance, phase 1 on a tie —
    /// because that is the surface the player is actually clipping or
    /// over-jumping. Two cases pin the rule:
    ///
    /// - A point clipped under a floor. Phase 1 finds that floor straight
    ///   overhead (`below_surface`). Phase 2's downward-biased box either
    ///   finds nothing, or — on a mesh with a lower storey nearby, as the
    ///   rebuilt Castle_CellBlock mesh has — finds that other storey a few
    ///   metres to the side. Reporting the latter (`horizontal`) or
    ///   nothing (`no_poly_in_extents`) would mislabel a floor-clip as a
    ///   mesh hole, which are the two failures an operator most needs to
    ///   tell apart.
    /// - A jump that is one unit too high beside a mezzanine. Phase 1
    ///   finds the mezzanine off to the side; phase 2 finds the floor
    ///   straight below, and `above_jump_tolerance` against that floor is
    ///   the useful answer.
    pub fn diagnose_point(&self, pos: &Vector3) -> PointVerdict {
        let phase1 = self
            .find_nearest_poly_with_extents(pos, &DEST_EXTENTS)
            .map(|(_, closest)| closest);
        if let Some(closest) = phase1 {
            if let Some(v) = self.verdict_against(pos, &closest) {
                if v.valid {
                    return v;
                }
            }
        }

        let biased_center = Vector3::new(pos.x, pos.y - JUMP_HEIGHT_TOLERANCE, pos.z);
        let phase2 = self
            .find_nearest_poly_with_extents(&biased_center, &JUMP_SEARCH_EXTENTS)
            .map(|(_, closest)| closest);

        let v1 = phase1.and_then(|closest| self.verdict_against(pos, &closest));
        let v2 = phase2.and_then(|closest| self.verdict_against(pos, &closest));
        match (v1, v2) {
            // Phase 1 was not valid (it returned early above otherwise),
            // so a valid phase 2 is the jump case and decides the answer.
            (_, Some(v2)) if v2.valid => v2,
            (Some(v1), Some(v2)) => {
                let h1 = v1.horizontal_dist.unwrap_or(f32::INFINITY);
                let h2 = v2.horizontal_dist.unwrap_or(f32::INFINITY);
                if h2 < h1 {
                    v2
                } else {
                    v1
                }
            }
            (Some(v), None) | (None, Some(v)) => v,
            (None, None) => PointVerdict::no_poly(),
        }
    }

    /// Build the verdict for `pos` against one candidate polygon point.
    /// Always evaluated against the *original* query point, regardless of
    /// which search phase located `closest`.
    ///
    /// `None` only when a coordinate is non-finite, which the movement
    /// validator rejects before it ever reaches the navmesh layer — but
    /// NPC path targets and content-authored coordinates reach here
    /// without that gate, and a NaN would otherwise make every
    /// comparison below `false` and silently read as "on the mesh".
    fn verdict_against(&self, pos: &Vector3, closest: &Vector3) -> Option<PointVerdict> {
        let dx = pos.x - closest.x;
        let dz = pos.z - closest.z;
        let dy = pos.y - closest.y;
        if !dx.is_finite() || !dz.is_finite() || !dy.is_finite() {
            return None;
        }
        // Squared comparison in the gate, sqrt only for the report: the
        // pre-diagnosis code compared squares, and moving the boundary
        // test onto `sqrt()` would shift the accept/reject decision for
        // points sitting exactly on the gate.
        let horizontal_sq = dx * dx + dz * dz;

        let gate = self.classify_containment(horizontal_sq, dy);
        Some(PointVerdict {
            valid: gate.is_none(),
            gate,
            horizontal_dist: Some(horizontal_sq.sqrt()),
            dy: Some(dy),
        })
    }

    /// The containment gates themselves, in evaluation order. `None`
    /// means the point is contained. `horizontal_sq` is the **squared**
    /// X/Z distance to the candidate polygon point.
    ///
    /// This is the single place the tolerances are compared, shared by
    /// the boolean and the diagnosis — see [`Self::is_point_valid`] for
    /// why the vertical gate is asymmetric.
    fn classify_containment(&self, horizontal_sq: f32, dy: f32) -> Option<NavGate> {
        let horizontal_gate = self.agent_radius * 2.0;
        if horizontal_sq >= horizontal_gate * horizontal_gate {
            return Some(NavGate::Horizontal);
        }
        let below_surface_gate = self.agent_radius * BELOW_SURFACE_TOLERANCE_FACTOR;
        if dy < -below_surface_gate {
            return Some(NavGate::BelowSurface);
        }
        if dy > JUMP_HEIGHT_TOLERANCE {
            return Some(NavGate::AboveJumpTolerance);
        }
        None
    }

    /// Find the closest valid navmesh position to the given point.
    pub fn get_nearest_point(&self, pos: &Vector3) -> Vector3 {
        self.find_nearest_poly(pos).map(|(_, p)| p).unwrap_or(*pos)
    }

    /// Sample the walkable surface height under `(x, z)` on the storey
    /// nearest `y_ref`.
    ///
    /// `y_ref` is the caller's current (or intended) Y. It is what makes
    /// this storey-aware: the search box is centred on it with a
    /// [`JUMP_HEIGHT_TOLERANCE`] half-height, so on a two-storey column an
    /// entity on the upper floor reads the upper floor. The function this
    /// replaced searched from world `Y = 0` with a ±500 box and returned
    /// whichever storey Detour judged nearest to the origin, which on
    /// `castle_cellblock`'s guard column is the floor ~68 units below the
    /// guard (NPC AI restoration audit M4).
    ///
    /// Returns `None` when there is no walkable surface within
    /// ±[`JUMP_HEIGHT_TOLERANCE`] of `y_ref` near `(x, z)`. That is not the
    /// same as "off-mesh": an entity floating more than that above its
    /// floor also reads `None`, so a caller logging the offset from ground
    /// must treat `None` as "no ground near this Y", not "no mesh here".
    pub fn get_height_near(&self, x: f32, y_ref: f32, z: f32) -> Option<f32> {
        let (poly_ref, nearest) =
            self.find_nearest_poly_with_extents(&Vector3::new(x, y_ref, z), &HEIGHT_NEAR_EXTENTS)?;

        // getPolyHeight picks the detail triangle from the query point, so
        // pass the XZ we want with the Y Detour already put on the poly.
        let query_pt = [x, nearest.y, z];
        let mut height: f32 = 0.0;
        let status = unsafe {
            detour_ffi::detour_get_poly_height(
                self.query,
                poly_ref as u32,
                query_pt.as_ptr(),
                &mut height,
            )
        };

        if dt_status_failed(status) {
            // (x, z) is outside the poly's XZ footprint (the box reached a
            // neighbour's edge); the clamped nearest point is the best Y.
            Some(nearest.y)
        } else {
            Some(height)
        }
    }
}

impl std::fmt::Debug for NavMesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavMesh")
            .field("name", &self.name)
            .field("hash", &self.fingerprint.short_hash)
            .field("polys", &self.fingerprint.npolys)
            .field("agent_height", &self.agent_height)
            .field("agent_radius", &self.agent_radius)
            .field("bmin", &self.bmin)
            .field("bmax", &self.bmax)
            .finish()
    }
}

#[cfg(test)]
mod line_of_sight_tests;
#[cfg(test)]
mod tests;

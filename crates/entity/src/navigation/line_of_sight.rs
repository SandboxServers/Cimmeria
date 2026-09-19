//! Navmesh line of sight: the Detour raycast and its three-state result.
//!
//! A navmesh raycast can only answer "is there a wall between these two
//! points" when both points are on the mesh. Off the mesh it has nothing to
//! walk, and the honest answer is "unknown", not "blocked". Collapsing the
//! two produced the same bug three times: a stationary NPC that holds fire
//! forever (`npc_ai` outcome `stationary_holds`) because the mesh did not
//! cover the tile it, or its target, was standing on.
//!
//! - Ambernol drone, a flyer hovering above the floor: 54 s of aggro with no
//!   shot fired. Fixed by projecting an off-mesh `start` onto the mesh.
//! - `castle_cellblock` NPC 100143 (2026-06-04): player in plain sight on a
//!   tile the mesh missed. Fixed by projecting `end` as well.
//! - `harset.nav` (found 2026-09-19 while applying the 2026-09-18 Castle
//!   playtest findings to Harset): the mesh is fragmented, and 9 of the 13
//!   stationary Harset mobs stand more than [`DEST_EXTENTS`] from any
//!   polygon, so projection cannot rescue them. [`NavMesh::raycast`] returns
//!   `false` for every one of them against every target.
//!
//! [`NavMesh::line_of_sight`] therefore reports [`LineOfSight::Unknown`] when
//! either endpoint cannot be projected, and the caller chooses the policy.
//! The cell's `SpaceManager::has_line_of_sight` treats unknown as clear,
//! which is what it already does for a space with no navmesh at all.

use cimmeria_common::Vector3;

use super::{NavMesh, DEST_EXTENTS, START_EXTENTS};
use crate::detour_ffi::{self, dt_status_failed};

/// What the navmesh can say about the straight line between two points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineOfSight {
    /// Both endpoints are on (or within [`DEST_EXTENTS`] of) the mesh and the
    /// ray reaches the end without crossing a mesh boundary.
    Clear,
    /// Both endpoints are on the mesh and the ray hits a boundary first.
    /// On a fragmented mesh a gap between islands is indistinguishable from
    /// a wall; only regenerating the mesh fixes that.
    Blocked,
    /// At least one endpoint is further than [`DEST_EXTENTS`] from every
    /// polygon. The mesh has no information about this line.
    Unknown,
}

impl LineOfSight {
    /// The policy for combat and targeting: only a positive "blocked" denies
    /// line of sight. "Unknown" counts as clear, matching a space that has no
    /// navmesh loaded. Denying on unknown makes any NPC standing off the mesh
    /// permanently unable to attack, and any player standing off the mesh
    /// permanently unattackable.
    pub fn is_clear_or_unknown(self) -> bool {
        !matches!(self, Self::Blocked)
    }
}

impl NavMesh {
    /// Line of sight from `start` to `end`, distinguishing "blocked" from
    /// "the mesh cannot tell". See the module docs for why the distinction
    /// exists.
    ///
    /// Both endpoints are projected onto the mesh first (the tight
    /// [`START_EXTENTS`] box for `start`, then [`DEST_EXTENTS`]; only
    /// [`DEST_EXTENTS`] for `end`) and the ray is cast between the projected
    /// points, so a flyer hovering over the floor or a player on an unmeshed
    /// crate still gets a real answer.
    pub fn line_of_sight(&self, start: &Vector3, end: &Vector3) -> LineOfSight {
        let Some((start_ref, projected_start)) = self.project_start(start) else {
            return LineOfSight::Unknown;
        };
        let Some((_, projected_end)) = self.project_to_polygon(end, &DEST_EXTENTS) else {
            return LineOfSight::Unknown;
        };
        if self.ray_reaches(start_ref, &projected_start, &projected_end) {
            LineOfSight::Clear
        } else {
            LineOfSight::Blocked
        }
    }

    /// Line-of-sight raycast from `start` to `end`.
    ///
    /// Returns `true` if the ray can travel from start to end without hitting
    /// a navmesh boundary. This is the strict form: an off-mesh `start`
    /// returns `false`, and an off-mesh `end` is cast to as a raw coordinate,
    /// which normally exits the mesh and also returns `false`. Use it when
    /// "cannot tell" must deny. Combat and targeting want
    /// [`Self::line_of_sight`] instead.
    ///
    /// **Off-mesh start projection.** The raycast requires a start polygon
    /// for Detour to walk the edges from. If `start` is off the navmesh
    /// (a flying NPC hovering above the floor; a player on a ledge barely
    /// outside the walkable surface), the tight `START_EXTENTS` lookup fails.
    /// We retry with the more generous `DEST_EXTENTS` (3-unit cube), and on
    /// success we raycast from the **projected** point rather than the
    /// original off-mesh coordinate.
    ///
    /// **Off-mesh end projection.** Symmetric: Detour exits the mesh at the
    /// boundary when `end` lies outside any walkable polygon and reports
    /// `t < 1.0`. We project `end` to its nearest poly within `DEST_EXTENTS`
    /// and raycast to that point; if no poly is in range we fall back to the
    /// raw `end`.
    ///
    /// Reverting either projection re-introduces the "stationary mob never
    /// fires" bug shape described in the module docs.
    pub fn raycast(&self, start: &Vector3, end: &Vector3) -> bool {
        let Some((start_ref, projected_start)) = self.project_start(start) else {
            return false; // truly off-mesh; nothing to raycast from
        };
        let end_pos = match self.project_to_polygon(end, &DEST_EXTENTS) {
            Some((_, projected_end)) => projected_end,
            None => [end.x, end.y, end.z],
        };
        self.ray_reaches(start_ref, &projected_start, &end_pos)
    }

    /// Project a ray origin: the tight box first (an agent standing on its
    /// polygon, the common case, and the cheaper lookup), then the wider one.
    fn project_start(&self, start: &Vector3) -> Option<(u32, [f32; 3])> {
        self.project_to_polygon(start, &START_EXTENTS)
            .or_else(|| self.project_to_polygon(start, &DEST_EXTENTS))
    }

    /// `true` when Detour's raycast travels from `start` (on `start_ref`) to
    /// `end` without hitting a mesh boundary.
    fn ray_reaches(&self, start_ref: u32, start: &[f32; 3], end: &[f32; 3]) -> bool {
        let mut hit_normal = [0.0f32; 3];
        let mut t: f32 = 0.0;
        let result = unsafe {
            detour_ffi::detour_raycast(
                self.query,
                start_ref,
                start.as_ptr(),
                end.as_ptr(),
                hit_normal.as_mut_ptr(),
                &mut t,
            )
        };
        // result == 1 means the ray reached `end` unblocked.
        result == 1
    }

    /// Find a polygon containing or near `pos`; return its ref and the
    /// projected-to-polygon point. `None` if Detour finds none within the
    /// requested extents box.
    fn project_to_polygon(&self, pos: &Vector3, extents: &[f32; 3]) -> Option<(u32, [f32; 3])> {
        let center = [pos.x, pos.y, pos.z];
        let mut poly_ref: u32 = 0;
        let mut projected = [0.0f32; 3];
        let status = unsafe {
            detour_ffi::detour_find_nearest_poly(
                self.query,
                center.as_ptr(),
                extents.as_ptr(),
                &mut poly_ref,
                projected.as_mut_ptr(),
            )
        };
        if dt_status_failed(status) || poly_ref == 0 {
            None
        } else {
            Some((poly_ref, projected))
        }
    }
}

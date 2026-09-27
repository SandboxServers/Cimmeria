//! Surface-constrained moves: slide a mover across the mesh toward a
//! point without leaving it, and land the result on the floor.
//!
//! The NPC AI has movers whose destination is a raw coordinate rather than
//! a routed path. The min-range backup is the worst of them: it steps away
//! from the target in a straight line, and before NA11 it kept the vertical
//! component of the target-to-NPC vector, so a player standing above the
//! NPC put the backup point under the floor (audit M5). Detour's
//! `moveAlongSurface` is the tool for this. It walks the polygons from the
//! start toward the end, stops at a wall, and never leaves the mesh.

use cimmeria_common::Vector3;

use super::{NavMesh, DEST_EXTENTS, START_EXTENTS};
use crate::detour_ffi::{self, dt_status_failed};

/// Polygons `moveAlongSurface` may visit. A backup is a few units long, so
/// this is generous; Detour stops early and reports what it reached when
/// the buffer fills.
const MAX_SURFACE_VISITED: usize = 32;

impl NavMesh {
    /// Move from `start` toward `end` across the walkable surface and return
    /// where the mover ends up, standing on the floor.
    ///
    /// The move is 2D in Detour: it follows the polygons under `start` in X
    /// and Z and stops at the first boundary edge (a wall, a ledge, the edge
    /// of the mesh). `end.y` is ignored: the result's Y is the surface
    /// height of the polygon the move ended on,
    /// because Detour does not project `resultPos` onto the surface itself
    /// (see `dtNavMeshQuery::moveAlongSurface`'s doc comment).
    ///
    /// `None` when `start` is not within [`DEST_EXTENTS`] of any polygon, so
    /// there is no surface to move along.
    pub fn move_along_surface(&self, start: &Vector3, end: &Vector3) -> Option<Vector3> {
        let (start_ref, start_pt) = self
            .find_nearest_poly_with_extents(start, &START_EXTENTS)
            .or_else(|| self.find_nearest_poly_with_extents(start, &DEST_EXTENTS))?;
        let start_pos = [start_pt.x, start_pt.y, start_pt.z];
        // Keep the end on the start's level so the XZ walk is not biased by
        // whatever height the caller's target had.
        let end_pos = [end.x, start_pt.y, end.z];

        let mut result = [0.0f32; 3];
        let mut visited = [0u32; MAX_SURFACE_VISITED];
        let mut visited_count: i32 = 0;
        let status = unsafe {
            detour_ffi::detour_move_along_surface(
                self.query,
                start_ref as u32,
                start_pos.as_ptr(),
                end_pos.as_ptr(),
                result.as_mut_ptr(),
                visited.as_mut_ptr(),
                &mut visited_count,
                MAX_SURFACE_VISITED as i32,
            )
        };
        if dt_status_failed(status) || visited_count <= 0 {
            return None;
        }

        // The last visited polygon is the one the result lies in.
        let last_ref = visited[(visited_count as usize).min(MAX_SURFACE_VISITED) - 1];
        let mut height = 0.0f32;
        let status = unsafe {
            detour_ffi::detour_get_poly_height(self.query, last_ref, result.as_ptr(), &mut height)
        };
        let y = if dt_status_failed(status) {
            // On the polygon's edge within float error: sample the storey
            // nearest the start instead, which is the one the walk stayed on.
            self.get_height_near(result[0], start_pt.y, result[2])
                .unwrap_or(start_pt.y)
        } else {
            height
        };
        Some(self.pathable_toward(start_pt, Vector3::new(result[0], y, result[2])))
    }

    /// `landed`, or the nearest point back toward `start` that the
    /// pathfinder can start from.
    ///
    /// A slide that stops against the outer edge of the mesh can end
    /// exactly on it, where Detour's BV-tree lookup misses the polygon it
    /// is on (seen on `castle_cellblock` at its +X bound). `find_path`
    /// looks the start polygon up in a ±0.5 box, so an NPC left there could
    /// never route again. Halve the move back toward `start` until the
    /// lookup succeeds; `start` itself always does.
    fn pathable_toward(&self, start: Vector3, landed: Vector3) -> Vector3 {
        let mut p = landed;
        for _ in 0..6 {
            if let Some((_, on_poly)) = self.find_nearest_poly_with_extents(&p, &START_EXTENTS) {
                return on_poly;
            }
            let (x, z) = ((start.x + p.x) * 0.5, (start.z + p.z) * 0.5);
            let y = self.get_height_near(x, start.y, z).unwrap_or(start.y);
            p = Vector3::new(x, y, z);
        }
        start
    }
}

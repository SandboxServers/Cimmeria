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
//! Module layout: the XRC binary-reader helpers and the header sanity caps
//! live in [`xrc`]; this module owns the [`NavMesh`] handle and its query API.

mod xrc;

use std::ffi::c_void;
use std::io::{BufReader, Read as IoRead};
use std::path::Path;

use cimmeria_common::Vector3;

use crate::detour_ffi::{self, dt_status_failed};

use xrc::{
    check_count, checked_alloc_size, read_f32, read_u16, read_u32, read_u8, MAX_DETAIL_NMESHES,
    MAX_DETAIL_NTRIS, MAX_DETAIL_NVERTS, MAX_NPOLYS, MAX_NVERTS, MAX_NVP,
};

// ── Maximum path sizes (matching C++ reference) ─────────────────────────

const MAX_POLY_PATH: i32 = 256;
const MAX_STRAIGHT_PATH: i32 = 256;

// ── Search extents (matching C++ NavigationQueryParams) ─────────────────

/// Tight extents for start position — entity should be standing on a poly.
const START_EXTENTS: [f32; 3] = [0.5, 0.5, 0.5];
/// Loose extents for destination — entity might be jumping, on a rail, etc.
const DEST_EXTENTS: [f32; 3] = [3.0, 3.0, 3.0];
/// Generous extents for height queries — large Y extent since caller
/// passes y=0 and the mesh could be at any elevation.
const HEIGHT_EXTENTS: [f32; 3] = [2.0, 500.0, 2.0];

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
    /// Polygon count from the XRC file (for diagnostics).
    npolys: u32,
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
    /// Load a navigation mesh from an XRC-format `.nav` file.
    ///
    /// Parses the XRC binary, builds a Detour navmesh tile, and initializes
    /// a query object. Follows the exact pipeline from the C++ reference
    /// implementation in `navigation.cpp`.
    pub fn load(path: &Path) -> cimmeria_common::Result<Self> {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let file = std::fs::File::open(path)?;
        let mut r = BufReader::new(file);

        // ── Section 1: Agent parameters ─────────────────────────────────
        let agent_height = read_f32(&mut r)?;
        let agent_climb = read_f32(&mut r)?;
        let agent_radius = read_f32(&mut r)?;

        // ── Section 2: Mesh metadata ────────────────────────────────────
        //
        // Every header count is validated against its `MAX_*` cap before the
        // matching allocation. A `.nav` file with `nverts = 0xFFFFFFFF` would
        // otherwise wrap `nverts * 3` to a 3-element `Vec<u16>` and the
        // following read loop would consume only three u16s while the rest
        // of the claimed `0xFFFFFFFF * 3` vertex region went phantom —
        // leaving every downstream section offset wrong and (on a u32
        // multiplication that does *not* wrap) demanding a 12 GB allocation
        // that crashes the server at startup. Operator-deployable input,
        // strict bounds check.
        let nverts = check_count(read_u32(&mut r)?, MAX_NVERTS, "nverts")?;
        let npolys = check_count(read_u32(&mut r)?, MAX_NPOLYS, "npolys")?;
        let nvp = check_count(read_u32(&mut r)?, MAX_NVP, "nvp")?;
        let _border_size = read_u32(&mut r)?;

        // ── Section 3: Grid config (quantization parameters) ────────────
        let cs = read_f32(&mut r)?;
        let ch = read_f32(&mut r)?;
        let bmin = [read_f32(&mut r)?, read_f32(&mut r)?, read_f32(&mut r)?];
        let bmax = [read_f32(&mut r)?, read_f32(&mut r)?, read_f32(&mut r)?];

        // ── Section 4: Quantized vertices ───────────────────────────────
        let verts_len = checked_alloc_size(nverts, 3, "nverts", "verts = nverts * 3 u16s")?;
        let mut verts = vec![0u16; verts_len];
        for v in &mut verts {
            *v = read_u16(&mut r)?;
        }

        // ── Section 5: Polygon connectivity ─────────────────────────────
        // Fold the `npolys * nvp * 2` product into one checked mul via
        // `npolys * (nvp * 2)`. `nvp` is already bounded by `MAX_NVP = 64`
        // so `nvp * 2` cannot overflow u32; `saturating_mul` is belt-and-
        // suspenders against future cap changes. The `field` slot stays
        // `"npolys"` (a real header field) and the multiplication shape
        // moves into `alloc_desc` so an operator seeing the error knows
        // both *which header field* and *which downstream allocation*
        // would have busted.
        let polys_len = checked_alloc_size(
            npolys,
            nvp.saturating_mul(2),
            "npolys",
            "polys = npolys * nvp * 2 u16s",
        )?;
        let mut polys = vec![0u16; polys_len];
        for p in &mut polys {
            *p = read_u16(&mut r)?;
        }

        // ── Sections 6-8: Regions, flags, areas ─────────────────────────
        // These are parallel `npolys`-length arrays (stride 1). `npolys`
        // is already capped by `check_count` above, so they're safe as
        // raw `as usize` today — but route them through `checked_alloc_size`
        // anyway. Defense in depth: if `MAX_NPOLYS` is ever raised, this
        // multiplication still gets checked, and the allocation pattern
        // stays uniform across every count-driven `Vec` in `NavMesh::load`.
        let regs_len = checked_alloc_size(npolys, 1, "npolys", "regs = npolys u16s")?;
        let mut regs = vec![0u16; regs_len];
        for v in &mut regs {
            *v = read_u16(&mut r)?;
        }
        let flags_len = checked_alloc_size(npolys, 1, "npolys", "flags = npolys u16s")?;
        let mut flags = vec![0u16; flags_len];
        for v in &mut flags {
            *v = read_u16(&mut r)?;
        }
        let areas_len = checked_alloc_size(npolys, 1, "npolys", "areas = npolys bytes")?;
        let mut areas = vec![0u8; areas_len];
        for v in &mut areas {
            *v = read_u8(&mut r)?;
        }

        // ── Sections 9-12: Detail mesh ──────────────────────────────────
        let detail_nmeshes = check_count(read_u32(&mut r)?, MAX_DETAIL_NMESHES, "detail_nmeshes")?;
        let detail_nverts = check_count(read_u32(&mut r)?, MAX_DETAIL_NVERTS, "detail_nverts")?;
        let detail_ntris = check_count(read_u32(&mut r)?, MAX_DETAIL_NTRIS, "detail_ntris")?;

        let detail_meshes_len = checked_alloc_size(
            detail_nmeshes,
            4,
            "detail_nmeshes",
            "detail_meshes = detail_nmeshes * 4 u32s",
        )?;
        let mut detail_meshes = vec![0u32; detail_meshes_len];
        for v in &mut detail_meshes {
            *v = read_u32(&mut r)?;
        }

        let detail_verts_len = checked_alloc_size(
            detail_nverts,
            3,
            "detail_nverts",
            "detail_verts = detail_nverts * 3 f32s",
        )?;
        let mut detail_verts = vec![0.0f32; detail_verts_len];
        for v in &mut detail_verts {
            *v = read_f32(&mut r)?;
        }

        let detail_tris_len = checked_alloc_size(
            detail_ntris,
            4,
            "detail_ntris",
            "detail_tris = detail_ntris * 4 bytes",
        )?;
        let mut detail_tris = vec![0u8; detail_tris_len];
        r.read_exact(&mut detail_tris)?;

        // ── Build Detour navmesh tile ───────────────────────────────────
        // This mirrors the C++ navigation.cpp lines 109-138 exactly:
        // populate dtNavMeshCreateParams and call dtCreateNavMeshData.
        let mut nav_data: *mut u8 = std::ptr::null_mut();
        let mut nav_data_size: i32 = 0;

        let build_ok = unsafe {
            detour_ffi::detour_build_navmesh_data(
                verts.as_ptr(),
                nverts as i32,
                polys.as_ptr(),
                npolys as i32,
                nvp as i32,
                flags.as_ptr(),
                areas.as_ptr(),
                bmin.as_ptr(),
                bmax.as_ptr(),
                cs,
                ch,
                agent_height,
                agent_radius,
                agent_climb,
                detail_meshes.as_ptr(),
                detail_nmeshes as i32,
                detail_verts.as_ptr(),
                detail_nverts as i32,
                detail_tris.as_ptr(),
                detail_ntris as i32,
                &mut nav_data,
                &mut nav_data_size,
            )
        };

        if build_ok == 0 || nav_data.is_null() {
            return Err(cimmeria_common::CimmeriaError::Entity(format!(
                "Failed to build Detour navmesh data for '{name}'"
            )));
        }

        // ── Init dtNavMesh ──────────────────────────────────────────────
        let mesh_handle = unsafe { detour_ffi::detour_create_navmesh(nav_data, nav_data_size) };

        // Free the intermediate tile data — detour_create_navmesh made its own copy
        unsafe {
            detour_ffi::detour_free_data(nav_data);
        }

        if mesh_handle.is_null() {
            return Err(cimmeria_common::CimmeriaError::Entity(format!(
                "Failed to create Detour navmesh for '{name}'"
            )));
        }

        // ── Init dtNavMeshQuery (2048 nodes, matching C++ reference) ────
        let query_handle = unsafe { detour_ffi::detour_create_query(mesh_handle, 2048) };

        if query_handle.is_null() {
            unsafe {
                detour_ffi::detour_free_navmesh(mesh_handle);
            }
            return Err(cimmeria_common::CimmeriaError::Entity(format!(
                "Failed to create Detour navmesh query for '{name}'"
            )));
        }

        tracing::info!(
            name = %name,
            nverts,
            npolys,
            nvp,
            detail_nmeshes,
            detail_nverts,
            detail_ntris,
            agent_height,
            agent_radius,
            "NavMesh loaded via Detour FFI"
        );

        Ok(NavMesh {
            query: query_handle,
            mesh: mesh_handle,
            name,
            npolys,
            agent_height,
            agent_radius,
            bmin,
            bmax,
        })
    }

    // ── Public query API ─────────────────────────────────────────────────

    /// Number of polygons in the navmesh.
    pub fn poly_count(&self) -> u32 {
        self.npolys
    }

    /// Find the nearest polygon to a point.
    /// Returns (polygon_ref_as_usize, closest_point_on_poly) or None.
    pub fn find_nearest_poly(&self, pos: &Vector3) -> Option<(usize, Vector3)> {
        let center = [pos.x, pos.y, pos.z];
        let mut nearest_ref: u32 = 0;
        let mut nearest_pt = [0.0f32; 3];

        let status = unsafe {
            detour_ffi::detour_find_nearest_poly(
                self.query,
                center.as_ptr(),
                DEST_EXTENTS.as_ptr(),
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
    pub fn is_point_valid(&self, pos: &Vector3) -> bool {
        if let Some((_, closest)) = self.find_nearest_poly(pos) {
            pos.distance_to(&closest) < self.agent_radius * 2.0
        } else {
            false
        }
    }

    /// Find the closest valid navmesh position to the given point.
    pub fn get_nearest_point(&self, pos: &Vector3) -> Vector3 {
        self.find_nearest_poly(pos).map(|(_, p)| p).unwrap_or(*pos)
    }

    /// Sample the navmesh surface height at the given XZ position.
    ///
    /// Finds the walkable polygon containing (x, z) and returns the
    /// interpolated Y height on that polygon's surface using the detail
    /// mesh for accuracy. Returns `None` if no walkable polygon is nearby.
    pub fn get_height_at(&self, x: f32, z: f32) -> Option<f32> {
        let center = [x, 0.0, z];
        let mut nearest_ref: u32 = 0;
        let mut nearest_pt = [0.0f32; 3];

        let status = unsafe {
            detour_ffi::detour_find_nearest_poly(
                self.query,
                center.as_ptr(),
                HEIGHT_EXTENTS.as_ptr(),
                &mut nearest_ref,
                nearest_pt.as_mut_ptr(),
            )
        };

        if dt_status_failed(status) || nearest_ref == 0 {
            return None;
        }

        // Use getPolyHeight for detail-mesh accuracy.
        // Pass the XZ we want but with the nearest_pt Y (which is on the poly),
        // because getPolyHeight needs a point that's actually near the polygon
        // in order to find the right detail triangle.
        let query_pt = [x, nearest_pt[1], z];
        let mut height: f32 = 0.0;
        let status = unsafe {
            detour_ffi::detour_get_poly_height(
                self.query,
                nearest_ref,
                query_pt.as_ptr(),
                &mut height,
            )
        };

        if dt_status_failed(status) {
            // Fall back to nearest poly point Y
            Some(nearest_pt[1])
        } else {
            Some(height)
        }
    }

    /// Line-of-sight raycast from `start` to `end`.
    ///
    /// Returns `true` if the ray can travel from start to end without hitting
    /// a navmesh boundary (i.e. there is clear line of sight).
    ///
    /// **Off-mesh start projection.** The raycast requires a start polygon
    /// for Detour to walk the edges from. If `start` is off the navmesh
    /// (a flying NPC hovering above the floor; a player on a ledge
    /// barely outside the walkable surface), the tight `START_EXTENTS`
    /// lookup fails, `start_ref == 0`, and the function would return
    /// `false` — appearing to the caller as "LoS blocked" even when no
    /// geometry actually intervenes. We retry with the more generous
    /// `DEST_EXTENTS` (3-unit cube), and on success we raycast from the
    /// **projected** point (the nearest valid polygon point) rather
    /// than the original off-mesh coordinate. This recovers LoS for
    /// `is_stationary = true` flyer NPCs whose `npc_ai_fight` tick
    /// silently skipped them every cycle.
    ///
    /// **Off-mesh end projection.** Symmetric to the start case: the
    /// Detour raycast walks navmesh polygons from `start_ref` toward
    /// `end_pos`. If `end` lies outside any walkable polygon (player
    /// standing on a crate the mesh doesn't cover, jumping past a
    /// stair edge, or briefly clipped above geometry), Detour exits
    /// the mesh at the boundary, reports `t < 1.0` and the function
    /// returns `false` — `has_los = false` for what is visually a
    /// clear shot. Stationary NPCs see this as "no LoS" and hold
    /// fire silently (`npc_ai::stationary_holds` log). We project
    /// `end` to its nearest poly within `DEST_EXTENTS` and raycast to
    /// **that** point; if no poly is in range (the target is genuinely
    /// far off-mesh — flying, in the sky, behind real geometry), we
    /// fall back to the original raw `end_pos` so unreachable targets
    /// still correctly fail.
    ///
    /// Reverting either fallback re-introduces a "stationary mob never
    /// fires" bug shape. Original observation: Ambernol drone (entity
    /// 100115) 54s aggro with zero `npc_ai.decision` events. End-side
    /// regression observed on castle_cellblock NPC 100143 (lomiada
    /// 2026-06-04 11:04:44–46): `dist_to_target=12.7–13.0m`,
    /// `max_range=30m`, `in_range=true`, `has_los=false`, two
    /// `stationary_holds` ticks back-to-back even though the player
    /// was in unobstructed sight (mesh just didn't cover the player's
    /// exact tile).
    pub fn raycast(&self, start: &Vector3, end: &Vector3) -> bool {
        // Try the tight extents first (matches the existing walking-NPC
        // shape — agent stands on the polygon, original position == the
        // polygon's closest point within 0.5u). Most NPCs and players
        // satisfy this and we save the wider lookup.
        let (start_ref, projected_start) = match self.project_to_polygon(start, &START_EXTENTS) {
            Some(v) => v,
            None => match self.project_to_polygon(start, &DEST_EXTENTS) {
                Some(v) => v,
                None => return false, // truly off-mesh; nothing to raycast from
            },
        };

        // Project `end` for the same reason: Detour's raycast halts at
        // the mesh boundary when `end` is off-poly, which is the
        // off-navmesh-target case described in the doc above. Only the
        // wider `DEST_EXTENTS` is used here — there is no "tight" case
        // worth distinguishing for the destination, and if the target
        // is more than ~3u from any walkable poly we want to fall back
        // to the raw end so genuinely unreachable targets still fail.
        let end_pos = match self.project_to_polygon(end, &DEST_EXTENTS) {
            Some((_, projected_end)) => projected_end,
            None => [end.x, end.y, end.z],
        };

        let mut hit_normal = [0.0f32; 3];
        let mut t: f32 = 0.0;

        let result = unsafe {
            detour_ffi::detour_raycast(
                self.query,
                start_ref,
                projected_start.as_ptr(),
                end_pos.as_ptr(),
                hit_normal.as_mut_ptr(),
                &mut t,
            )
        };

        // result == 1 means ray reached endPos unblocked
        result == 1
    }

    /// Helper: find a polygon containing or near `pos`, return its ref
    /// and the projected-to-polygon point. Returns `None` if Detour
    /// can't find one within the requested extents box.
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

    /// Find a path from `start` to `end` across the navigation mesh.
    ///
    /// Returns a sequence of world-space waypoints forming a walkable path,
    /// or `None` if no path exists. Uses Detour's A* pathfinder followed
    /// by straight-path simplification.
    pub fn find_path(&self, start: &Vector3, end: &Vector3) -> Option<Vec<Vector3>> {
        let start_pos = [start.x, start.y, start.z];
        let end_pos = [end.x, end.y, end.z];

        // Find start polygon (tight extents — entity should be on a poly)
        let mut start_ref: u32 = 0;
        let mut start_pt = [0.0f32; 3];
        let status = unsafe {
            detour_ffi::detour_find_nearest_poly(
                self.query,
                start_pos.as_ptr(),
                START_EXTENTS.as_ptr(),
                &mut start_ref,
                start_pt.as_mut_ptr(),
            )
        };
        if dt_status_failed(status) || start_ref == 0 {
            tracing::warn!(?start, "NavMesh::find_path: no start poly for position");
            return None;
        }

        // Find end polygon (loose extents — destination may be approximate)
        let mut end_ref: u32 = 0;
        let mut end_pt = [0.0f32; 3];
        let status = unsafe {
            detour_ffi::detour_find_nearest_poly(
                self.query,
                end_pos.as_ptr(),
                DEST_EXTENTS.as_ptr(),
                &mut end_ref,
                end_pt.as_mut_ptr(),
            )
        };
        if dt_status_failed(status) || end_ref == 0 {
            tracing::warn!(?end, "NavMesh::find_path: no end poly for position");
            return None;
        }

        // Find polygon corridor via A*
        let mut poly_path = vec![0u32; MAX_POLY_PATH as usize];
        let mut path_count: i32 = 0;
        let status = unsafe {
            detour_ffi::detour_find_path(
                self.query,
                start_ref,
                end_ref,
                start_pt.as_ptr(),
                end_pt.as_ptr(),
                poly_path.as_mut_ptr(),
                &mut path_count,
                MAX_POLY_PATH,
            )
        };
        if dt_status_failed(status) || path_count == 0 {
            tracing::debug!(?start, ?end, "NavMesh::find_path: no poly path found");
            return None;
        }

        // Convert polygon corridor to straight-line waypoints
        let mut straight_path = vec![0.0f32; (MAX_STRAIGHT_PATH * 3) as usize];
        let mut straight_count: i32 = 0;
        let status = unsafe {
            detour_ffi::detour_find_straight_path(
                self.query,
                start_pt.as_ptr(),
                end_pt.as_ptr(),
                poly_path.as_ptr(),
                path_count,
                straight_path.as_mut_ptr(),
                &mut straight_count,
                MAX_STRAIGHT_PATH,
            )
        };
        if dt_status_failed(status) || straight_count == 0 {
            tracing::debug!(
                ?start,
                ?end,
                "NavMesh::find_path: straight path failed, returning endpoints"
            );
            // Fallback: return direct start→end (Detour found a poly path
            // but couldn't straighten it — shouldn't happen normally)
            return Some(vec![
                Vector3::new(start_pt[0], start_pt[1], start_pt[2]),
                Vector3::new(end_pt[0], end_pt[1], end_pt[2]),
            ]);
        }

        let mut waypoints = Vec::with_capacity(straight_count as usize);
        for i in 0..straight_count as usize {
            waypoints.push(Vector3::new(
                straight_path[i * 3],
                straight_path[i * 3 + 1],
                straight_path[i * 3 + 2],
            ));
        }

        Some(waypoints)
    }
}

impl std::fmt::Debug for NavMesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavMesh")
            .field("name", &self.name)
            .field("agent_height", &self.agent_height)
            .field("agent_radius", &self.agent_radius)
            .field("bmin", &self.bmin)
            .field("bmax", &self.bmax)
            .finish()
    }
}

#[cfg(test)]
mod tests;

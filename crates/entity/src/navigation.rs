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

use std::ffi::c_void;
use std::io::{BufReader, Read as IoRead};
use std::path::Path;

use cimmeria_common::Vector3;

use crate::detour_ffi::{self, dt_status_failed};

// ── XRC binary reader helpers ────────────────────────────────────────────

fn read_f32(r: &mut impl IoRead) -> std::io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_u32(r: &mut impl IoRead) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u16(r: &mut impl IoRead) -> std::io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u8(r: &mut impl IoRead) -> std::io::Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

// ── XRC header sanity caps ──────────────────────────────────────────────
//
// These mirror the caps in `cimmeria_navmesh_extractor::nav_roundtrip` so a
// hostile `.nav` file is rejected with the same diagnostic whether it reaches
// the build-time tool or the runtime loader. Castle Cellblock — the largest
// shipped navmesh at PR-time — has `nverts = 2778`, `npolys = 1479`,
// `detail_nverts = 6031`, `detail_ntris = 3102`. The caps below leave
// 360× to 1500× headroom over real assets while still bounding the worst-case
// allocation to a few hundred MB instead of a u32-multiplication wrap that
// produces a tiny `Vec` the read loop then walks past.

/// Upper bound on `nverts` accepted from a `.nav` header.
///
/// Worst-case `verts` allocation at this cap is
/// `1_000_000 * 3 * size_of::<u16>() = ~6 MB`.
const MAX_NVERTS: u32 = 1_000_000;

/// Upper bound on `npolys`.
const MAX_NPOLYS: u32 = 1_000_000;

/// Upper bound on `nvp` (max vertices per polygon). Recast's poly mesh
/// uses values in the 3..=12 range in practice; cap at 64 as a sanity
/// limit. Combined with [`MAX_NPOLYS`], the worst-case `polys` Vec is
/// `1_000_000 * 64 * 2 * 2 = ~256 MB`.
const MAX_NVP: u32 = 64;

/// Upper bound on `detail_nmeshes`. Per-poly mesh count, so practically
/// `<= npolys`. Cap at the same 1M to avoid coupling the checks.
const MAX_DETAIL_NMESHES: u32 = 1_000_000;

/// Upper bound on `detail_nverts`. Castle Cellblock: 6031.
const MAX_DETAIL_NVERTS: u32 = 10_000_000;

/// Upper bound on `detail_ntris`. Castle Cellblock: 3102.
const MAX_DETAIL_NTRIS: u32 = 10_000_000;

/// Validate a header count against its documented maximum and return the
/// value on success. Used by [`NavMesh::load`] to reject hostile inputs
/// before they reach a multiplication that could overflow into a too-small
/// `Vec` allocation.
fn check_count(value: u32, max: u32, field: &'static str) -> cimmeria_common::Result<u32> {
    if value > max {
        return Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange {
            field,
            value: value as u64,
            reason: "exceeds runtime sanity cap",
        });
    }
    Ok(value)
}

/// Compute `count * stride` as `usize`, failing with a descriptive error
/// on overflow. Both inputs are passed as `u32` to match the on-disk
/// header types; the multiplication is performed in `u64` so the failure
/// mode is the same on 32-bit and 64-bit targets.
///
/// `field` names the header-count source of the multiplication (always a
/// real header field — e.g. `"nverts"`, not a compound expression). The
/// caller passes `alloc_desc` to describe the multiplication shape so the
/// error names *which* downstream allocation would have busted (e.g.
/// `"polys = npolys * nvp * 2 u16s"`). On overflow we widen `value` to
/// report whichever number the operator can still diagnose: for the u64
/// `checked_mul` failure that's the raw count (the product is by
/// definition unrepresentable in u64); for the `usize::try_from` failure
/// the product fits in u64 and is reported as-is.
fn checked_alloc_size(
    count: u32,
    stride: u32,
    field: &'static str,
    alloc_desc: &'static str,
) -> cimmeria_common::Result<usize> {
    let product = (count as u64).checked_mul(stride as u64).ok_or(
        cimmeria_common::CimmeriaError::NavHeaderOutOfRange {
            field,
            value: count as u64,
            reason: alloc_desc,
        },
    )?;
    usize::try_from(product).map_err(|_| cimmeria_common::CimmeriaError::NavHeaderOutOfRange {
        field,
        value: product,
        reason: alloc_desc,
    })
}

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
    /// Reverting either fallback re-introduces the "drone in Fighting
    /// state never fires" bug shape observed in the SigNoz logs for
    /// the Ambernol encounter (entity 100115, instance 65552): 54s of
    /// aggro with zero `npc_ai.decision` events because every tick
    /// landed in the stationary-no-LoS silent return.
    pub fn raycast(&self, start: &Vector3, end: &Vector3) -> bool {
        let end_pos = [end.x, end.y, end.z];

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
mod tests {
    use super::*;

    #[test]
    fn load_castle_cellblock_nav() {
        let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
        if !path.exists() {
            // Skip test if data file not present (CI)
            return;
        }
        let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");
        assert!(mesh.agent_height > 0.0);
        assert!(mesh.agent_radius > 0.0);

        // The guard spawn position should be on the navmesh
        let guard_pos = Vector3::new(-289.465, 68.542, -154.276);
        assert!(
            mesh.is_point_valid(&guard_pos),
            "Guard spawn position should be on navmesh"
        );
    }

    #[test]
    fn load_and_pathfind_castle_cellblock() {
        let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
        if !path.exists() {
            return;
        }
        let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

        // Guard patrol: find path between two known-good positions
        let start = Vector3::new(-289.465, 68.542, -154.276);
        let end = Vector3::new(-280.0, 68.0, -150.0);
        let path_result = mesh.find_path(&start, &end);
        assert!(
            path_result.is_some(),
            "Should find path between nearby points on navmesh"
        );
        let waypoints = path_result.unwrap();
        assert!(
            waypoints.len() >= 2,
            "Path should have at least start and end"
        );
    }

    #[test]
    fn load_and_raycast_castle_cellblock() {
        let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
        if !path.exists() {
            return;
        }
        let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

        let start = Vector3::new(-289.465, 68.542, -154.276);
        let end = Vector3::new(-280.0, 68.0, -150.0);

        // Nearby points should have line of sight
        let has_los = mesh.raycast(&start, &end);
        // We can't assert this is true without knowing the mesh geometry,
        // but at least verify it doesn't crash
        tracing::info!("Raycast result: {has_los}");
    }

    /// Regression guard: a start position raised above the navmesh
    /// (a "flyer" NPC hovering ≥0.5u over the walkable surface) must
    /// still resolve to a valid polygon for the raycast. Pre-fix, the
    /// tight `START_EXTENTS = [0.5, 0.5, 0.5]` lookup failed for any
    /// hover height beyond ~half a unit and `raycast` returned
    /// `false` without ever shooting the ray — surfacing as
    /// stationary flyer NPCs (e.g., the Ambernol drone, body_set
    /// `BS_MOB_DroneFlyer`) never firing their abilities because
    /// `npc_ai_fight`'s `!has_los` branch silent-skipped every
    /// tick.
    ///
    /// Without the navmesh fixture (CI), the test self-skips per the
    /// repo's standard `if !path.exists()` pattern.
    #[test]
    fn raycast_with_off_mesh_start_projects_to_polygon() {
        let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
        if !path.exists() {
            return;
        }
        let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

        // Pick a known-on-navmesh ground reference (same one the
        // sibling tests use) then lift the start up by 2.5 units —
        // well past `START_EXTENTS = 0.5` but within `DEST_EXTENTS = 3.0`.
        // A flyer drone at body_set hover offset typically sits 0.2–
        // 1.5u above the floor; 2.5u is a deliberately pessimistic
        // case to prove the wider fallback also catches it.
        let ground = Vector3::new(-289.465, 68.542, -154.276);
        let off_mesh = Vector3::new(ground.x, ground.y + 2.5, ground.z);
        let nearby_target = Vector3::new(-280.0, 68.0, -150.0);

        // Pre-fix bug: this returns `false` even though the ground point
        // RIGHT BELOW `off_mesh` has clear LoS to `nearby_target`. The
        // projection-to-polygon retry recovers the correct result.
        let los_off_mesh = mesh.raycast(&off_mesh, &nearby_target);
        let los_on_mesh = mesh.raycast(&ground, &nearby_target);
        assert_eq!(
            los_off_mesh, los_on_mesh,
            "off-mesh start ({:?}) must produce the same LoS result as the \
             on-mesh point directly below it ({:?}) — the projection-to-polygon \
             retry was added to recover from the START_EXTENTS=0.5 lookup \
             failing on flyer NPC positions",
            off_mesh, ground
        );
    }

    // ── Hostile-input regression guards ────────────────────────────────
    //
    // Each of the five tests below synthesises a `.nav` header in which
    // exactly one count field is poisoned with `0xFFFFFFFF` and the rest
    // are zero, writes it to a temp file, and asserts that `NavMesh::load`
    // rejects it with `CimmeriaError::NavHeaderOutOfRange` naming the
    // offending field. Reverting any single bounds check in `load`
    // (e.g., dropping the `check_count` wrapping `nverts`) causes the
    // matching test to fail by trying to allocate / read past EOF instead.
    //
    // Layout written below (mirrors `NavMesh::load` exactly through the
    // first allocation site of each section; later sections need only
    // enough header bytes for the cap to trip before the alloc):
    //
    //   agent_height/climb/radius   3 × f32     (12 bytes)
    //   nverts/npolys/nvp/border    4 × u32     (16 bytes)
    //   cs/ch                       2 × f32     (8  bytes)
    //   bmin/bmax                   6 × f32     (24 bytes)
    //   ...sections beyond this only need their leading count headers...

    /// Construct a unique temp-file path so concurrent test runs don't
    /// clobber each other. We deliberately don't use the `tempfile`
    /// crate to keep `cimmeria-entity` dep-free at this layer.
    ///
    /// Suffix combines pid + thread id + nanosecond timestamp — matching
    /// the navmesh-extractor's `tempdir()` helper. Cargo's default test
    /// runner parallelises by default, so naming the file by pid+nanos
    /// alone would race two threads into the same path on fast hardware
    /// where multiple tests start within the same nanosecond.
    fn make_tmp_nav_path(suffix: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let tid = std::thread::current().id();
        let mut p = std::env::temp_dir();
        p.push(format!(
            "cimmeria_navmesh_load_{pid}_{tid:?}_{nanos}_{suffix}.nav"
        ));
        p
    }

    /// Build a minimal header with all counts zero, then let the caller
    /// overwrite a single 4-byte slot with the hostile value. Returns
    /// a byte vector long enough to reach the detail-section header so
    /// any of the five caps can trip without truncated-read confusion.
    fn synthesise_header() -> Vec<u8> {
        let mut buf = Vec::new();
        // agent_height/climb/radius — values don't matter, just non-NaN.
        buf.extend_from_slice(&0.6_f32.to_le_bytes());
        buf.extend_from_slice(&0.9_f32.to_le_bytes());
        buf.extend_from_slice(&0.6_f32.to_le_bytes());
        // nverts/npolys/nvp/border_size — start all zero so the test can
        // overwrite exactly one.
        buf.extend_from_slice(&0_u32.to_le_bytes());
        buf.extend_from_slice(&0_u32.to_le_bytes());
        buf.extend_from_slice(&6_u32.to_le_bytes()); // nvp can't be 0 if the test wants
                                                     // to reach detail headers via valid `polys`
                                                     // sizing — but with npolys=0 the alloc is
                                                     // zero-sized regardless. Use a sane default.
        buf.extend_from_slice(&0_u32.to_le_bytes()); // border_size
                                                     // cs/ch
        buf.extend_from_slice(&0.3_f32.to_le_bytes());
        buf.extend_from_slice(&0.2_f32.to_le_bytes());
        // bmin/bmax — six f32s.
        for _ in 0..6 {
            buf.extend_from_slice(&0.0_f32.to_le_bytes());
        }
        // verts/polys/regs/flags/areas are all zero-sized when counts are
        // zero, so the read cursor lands directly on the detail header.
        // detail_nmeshes/detail_nverts/detail_ntris — three u32 slots.
        buf.extend_from_slice(&0_u32.to_le_bytes());
        buf.extend_from_slice(&0_u32.to_le_bytes());
        buf.extend_from_slice(&0_u32.to_le_bytes());
        buf
    }

    /// Byte offsets of each header count field inside `synthesise_header()`'s
    /// output. Encoded as constants so a layout change here forces a
    /// matching update in the tests below.
    // Byte budget: 3 × f32 (12) + 4 × u32 (16) + 2 × f32 (8) + 6 × f32 (24)
    // = 60 bytes before the detail header. detail_* occupies bytes 60..72.
    const OFFSET_NVERTS: usize = 12;
    const OFFSET_NPOLYS: usize = 16;
    const OFFSET_NVP: usize = 20;
    const OFFSET_DETAIL_NMESHES: usize = 60;
    const OFFSET_DETAIL_NVERTS: usize = 64;
    const OFFSET_DETAIL_NTRIS: usize = 68;

    fn write_hostile_at(field_offset: usize) -> std::path::PathBuf {
        use std::io::Write;
        let mut buf = synthesise_header();
        let bytes = 0xFFFF_FFFF_u32.to_le_bytes();
        buf[field_offset..field_offset + 4].copy_from_slice(&bytes);
        let path = make_tmp_nav_path(&format!("hostile_{field_offset}"));
        let mut f = std::fs::File::create(&path).expect("create tmp nav");
        f.write_all(&buf).expect("write tmp nav");
        path
    }

    fn assert_hostile_field(path: &std::path::Path, expected_field: &str) {
        let result = NavMesh::load(path);
        // Clean up the temp file before asserting so a failed assertion
        // doesn't leave litter under $TMP.
        let _ = std::fs::remove_file(path);
        match result {
            Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange { field, .. }) => {
                assert_eq!(
                    field, expected_field,
                    "wrong field flagged in NavHeaderOutOfRange"
                );
            }
            other => panic!(
                "expected NavHeaderOutOfRange({expected_field}), got {other:?}\n\
                 If this regressed: the bounds check on `{expected_field}` in \
                 NavMesh::load was reverted; restore the check_count call."
            ),
        }
    }

    /// Hostile `nverts` must trip `MAX_NVERTS` before `vec![0u16; (nverts * 3)]`
    /// wraps to a 3-element Vec (or the unwrapped 12 GB allocation crashes
    /// the server).
    #[test]
    fn navmesh_load_rejects_oversized_nverts() {
        let path = write_hostile_at(OFFSET_NVERTS);
        assert_hostile_field(&path, "nverts");
    }

    /// Hostile `npolys` must trip `MAX_NPOLYS` before the
    /// `npolys * nvp * 2` multiplication can overflow.
    #[test]
    fn navmesh_load_rejects_oversized_npolys() {
        let path = write_hostile_at(OFFSET_NPOLYS);
        assert_hostile_field(&path, "npolys");
    }

    /// Hostile `nvp` must trip `MAX_NVP = 64`. Without this check, a `.nav`
    /// with `nvp = 0xFFFFFFFF` and `npolys = 1` would still pass through
    /// the `npolys` and `nverts` caps but the `polys_len` multiplication
    /// would overflow u32 (or even u64 if we hadn't widened first).
    #[test]
    fn navmesh_load_rejects_oversized_nvp() {
        let path = write_hostile_at(OFFSET_NVP);
        assert_hostile_field(&path, "nvp");
    }

    /// Hostile `detail_nmeshes` must trip the detail-section cap.
    #[test]
    fn navmesh_load_rejects_oversized_detail_nmeshes() {
        let path = write_hostile_at(OFFSET_DETAIL_NMESHES);
        assert_hostile_field(&path, "detail_nmeshes");
    }

    /// Hostile `detail_nverts` must trip the detail-section cap.
    #[test]
    fn navmesh_load_rejects_oversized_detail_nverts() {
        let path = write_hostile_at(OFFSET_DETAIL_NVERTS);
        assert_hostile_field(&path, "detail_nverts");
    }

    /// Hostile `detail_ntris` must trip the detail-section cap.
    #[test]
    fn navmesh_load_rejects_oversized_detail_ntris() {
        let path = write_hostile_at(OFFSET_DETAIL_NTRIS);
        assert_hostile_field(&path, "detail_ntris");
    }

    #[test]
    fn load_and_height_query() {
        let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
        if !path.exists() {
            return;
        }
        let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

        // Query height at the guard position XZ.
        // The navmesh is in Recast/BigWorld coordinate space — Y values won't
        // match UE3 game coordinates directly. Just verify we get a result and
        // that it's a finite number within the mesh bounds.
        let height = mesh.get_height_at(-289.465, -154.276);
        assert!(
            height.is_some(),
            "Should find height at guard spawn XZ position"
        );
        let h = height.unwrap();
        assert!(h.is_finite(), "Height should be finite, got {h}");
        // Height should be between the mesh's Y bounds (with some tolerance)
        assert!(
            h >= mesh.bmin[1] - 1.0 && h <= mesh.bmax[1] + 1.0,
            "Height {h} should be within mesh Y bounds [{}, {}]",
            mesh.bmin[1],
            mesh.bmax[1]
        );
    }
}

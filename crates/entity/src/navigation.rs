//! Server-side navigation mesh loading and queries.
//!
//! Loads XRC-format `.nav` files (custom Recast output from the Cimmeria
//! NavBuilder) and provides pathfinding, line-of-sight raycasting, and
//! point-validation queries used by NPC AI and movement systems.
//!
//! The XRC format stores a single-tile Recast polygon mesh with detail
//! triangulation. This module implements Detour-equivalent query algorithms
//! (nearest poly, raycast, A* pathfinding, funnel straightening) directly
//! on the parsed mesh data.
//!
//! Reference: `external/recast/Detour/Source/DetourNavMeshQuery.cpp`
//! Reference: `tools/SceneEditor/src/commands/navmesh.rs` (XRC parser)

use std::io::{BufReader, Read as IoRead};
use std::path::Path;

use cimmeria_common::Vector3;

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

// ── Polygon mesh ─────────────────────────────────────────────────────────

/// A polygon in the navigation mesh. Up to 6 vertices (Detour max).
#[derive(Debug)]
struct Poly {
    /// Indices into NavMesh::verts (dequantized world-space vertices).
    vert_indices: Vec<u16>,
    /// Neighbor polygon index per edge, or NO_NEIGHBOR if boundary.
    neighbors: Vec<u16>,
    /// Walkable area ID (0 = blocked).
    area: u8,
}

const NO_NEIGHBOR: u16 = 0xFFFF;

/// A loaded navigation mesh for a single game space.
///
/// Provides pathfinding, line-of-sight raycasting, and point validation
/// queries. Loaded from XRC-format `.nav` files produced by NavBuilder.
pub struct NavMesh {
    /// Human-readable label (space name).
    name: String,
    /// Dequantized world-space vertices (x, y, z triples).
    verts: Vec<[f32; 3]>,
    /// Polygons with vertex indices, neighbor links, and area IDs.
    polys: Vec<Poly>,
    /// Max vertices per polygon (typically 6).
    nvp: u32,
    /// Agent configuration from the navmesh.
    pub agent_height: f32,
    pub agent_radius: f32,
    /// World-space bounds.
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
}

impl NavMesh {
    /// Load a navigation mesh from an XRC-format `.nav` file.
    pub fn load(path: &Path) -> cimmeria_common::Result<Self> {
        let name = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let file = std::fs::File::open(path)?;
        let mut r = BufReader::new(file);

        // Section 1: Agent parameters
        let agent_height = read_f32(&mut r)?;
        let _agent_climb = read_f32(&mut r)?;
        let agent_radius = read_f32(&mut r)?;

        // Section 2: Mesh metadata
        let nverts = read_u32(&mut r)?;
        let npolys = read_u32(&mut r)?;
        let nvp = read_u32(&mut r)?;
        let _border_size = read_u32(&mut r)?;

        // Section 3: Grid config (quantization parameters)
        let cs = read_f32(&mut r)?;
        let ch = read_f32(&mut r)?;
        let bmin = [read_f32(&mut r)?, read_f32(&mut r)?, read_f32(&mut r)?];
        let bmax = [read_f32(&mut r)?, read_f32(&mut r)?, read_f32(&mut r)?];

        // Section 4: Quantized vertices
        let mut raw_verts = vec![0u16; (nverts * 3) as usize];
        for v in &mut raw_verts { *v = read_u16(&mut r)?; }

        // Dequantize vertices to world space
        let mut verts = Vec::with_capacity(nverts as usize);
        for i in 0..nverts as usize {
            verts.push([
                bmin[0] + raw_verts[i * 3] as f32 * cs,
                bmin[1] + raw_verts[i * 3 + 1] as f32 * ch,
                bmin[2] + raw_verts[i * 3 + 2] as f32 * cs,
            ]);
        }

        // Section 5: Polygon connectivity (nvp vertex indices + nvp neighbor indices per poly)
        let mut raw_polys = vec![0u16; (npolys * nvp * 2) as usize];
        for p in &mut raw_polys { *p = read_u16(&mut r)?; }

        // Section 6-8: Region IDs, flags, areas
        // Skip regions and flags (not needed for queries)
        for _ in 0..npolys { let _ = read_u16(&mut r)?; }
        for _ in 0..npolys { let _ = read_u16(&mut r)?; }
        let mut areas = vec![0u8; npolys as usize];
        for v in &mut areas { *v = read_u8(&mut r)?; }

        // Build polygon structures with adjacency
        let mut polys = Vec::with_capacity(npolys as usize);
        for i in 0..npolys as usize {
            let base = i * nvp as usize * 2;

            // First nvp entries: vertex indices (0xFFFF = unused slot)
            let mut vert_indices = Vec::new();
            for j in 0..nvp as usize {
                let vi = raw_polys[base + j];
                if vi == 0xFFFF { break; }
                vert_indices.push(vi);
            }

            // Second nvp entries: neighbor polygon indices (0xFFFF = no neighbor)
            let nv = vert_indices.len();
            let mut neighbors = Vec::with_capacity(nv);
            for j in 0..nv {
                neighbors.push(raw_polys[base + nvp as usize + j]);
            }

            polys.push(Poly {
                vert_indices,
                neighbors,
                area: areas[i],
            });
        }

        tracing::info!(
            name = %name,
            nverts, npolys, nvp,
            agent_height, agent_radius,
            "NavMesh loaded"
        );

        Ok(NavMesh {
            name,
            verts,
            polys,
            nvp,
            agent_height,
            agent_radius,
            bmin,
            bmax,
        })
    }

    /// Number of polygons in the navmesh.
    pub fn poly_count(&self) -> usize {
        self.polys.len()
    }

    /// Number of vertices in the navmesh.
    pub fn vert_count(&self) -> usize {
        self.verts.len()
    }

    /// Get the polygon vertex positions for a given polygon index.
    fn poly_verts(&self, poly_idx: usize) -> Vec<[f32; 3]> {
        self.polys[poly_idx].vert_indices.iter()
            .map(|&vi| self.verts[vi as usize])
            .collect()
    }

    // ── 2D geometry helpers (XZ plane, Y is height) ──────────────────────

    /// Signed 2D triangle area on the XZ plane (Detour's dtTriArea2D).
    /// Positive = left turn, negative = right turn.
    fn tri_area_2d(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
        let abx = b[0] - a[0];
        let abz = b[2] - a[2];
        let acx = c[0] - a[0];
        let acz = c[2] - a[2];
        acx * abz - abx * acz
    }

    /// Test if point p is inside polygon (XZ plane) using ray crossing.
    /// Returns (inside, per-edge squared distances, per-edge t parameters).
    fn point_in_poly_xz(p: [f32; 3], verts: &[[f32; 3]]) -> bool {
        let n = verts.len();
        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let vi = verts[i];
            let vj = verts[j];
            if ((vi[2] > p[2]) != (vj[2] > p[2]))
                && (p[0] < (vj[0] - vi[0]) * (p[2] - vi[2]) / (vj[2] - vi[2]) + vi[0])
            {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    /// Squared distance from point to line segment in 2D (XZ plane).
    /// Returns (dist_sq, t_parameter).
    fn dist_pt_seg_sq_2d(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> (f32, f32) {
        let dx = b[0] - a[0];
        let dz = b[2] - a[2];
        let px = p[0] - a[0];
        let pz = p[2] - a[2];
        let d = dx * dx + dz * dz;
        let mut t = dx * px + dz * pz;
        if d > 0.0 {
            t /= d;
        }
        t = t.clamp(0.0, 1.0);
        let qx = a[0] + t * dx - p[0];
        let qz = a[2] + t * dz - p[2];
        (qx * qx + qz * qz, t)
    }

    /// 2D ray-polygon intersection (Detour's dtIntersectSegmentPoly2D).
    /// Tests ray from p0 to p1 against polygon edges.
    /// Returns (intersects, tmin, tmax, seg_min_edge, seg_max_edge).
    fn intersect_seg_poly_2d(
        p0: [f32; 3],
        p1: [f32; 3],
        verts: &[[f32; 3]],
    ) -> (bool, f32, f32, i32, i32) {
        let n = verts.len();
        let mut tmin = 0.0f32;
        let mut tmax = 1.0f32;
        let mut seg_min: i32 = -1;
        let mut seg_max: i32 = -1;

        let dx = p1[0] - p0[0];
        let dz = p1[2] - p0[2];

        let mut j = n - 1;
        for i in 0..n {
            let ex = verts[i][0] - verts[j][0];
            let ez = verts[i][2] - verts[j][2];
            let nx = p0[0] - verts[j][0];
            let nz = p0[2] - verts[j][2];

            let d = ez * dx - ex * dz;
            let n_val = ez * nx - ex * nz;

            if d.abs() < 1e-8 {
                // Parallel
                if n_val < 0.0 {
                    return (false, tmin, tmax, seg_min, seg_max);
                }
            } else {
                let t = n_val / d;
                if d < 0.0 {
                    // Entry
                    if t > tmin {
                        tmin = t;
                        seg_min = i as i32;
                        if tmin > tmax {
                            return (false, tmin, tmax, seg_min, seg_max);
                        }
                    }
                } else {
                    // Exit
                    if t < tmax {
                        tmax = t;
                        seg_max = i as i32;
                        if tmax < tmin {
                            return (false, tmin, tmax, seg_min, seg_max);
                        }
                    }
                }
            }
            j = i;
        }

        (true, tmin, tmax, seg_min, seg_max)
    }

    // ── Public query API ─────────────────────────────────────────────────

    /// Find the nearest polygon to a point.
    /// Returns (polygon_index, closest_point_on_poly) or None.
    pub fn find_nearest_poly(&self, pos: &Vector3) -> Option<(usize, Vector3)> {
        let p = [pos.x, pos.y, pos.z];
        let mut best_dist = f32::MAX;
        let mut best_poly = None;
        let mut best_point = p;

        for (idx, poly) in self.polys.iter().enumerate() {
            if poly.area == 0 { continue; } // Skip non-walkable

            let pverts = self.poly_verts(idx);
            let cp = self.closest_point_on_poly(&pverts, p);
            let dx = cp[0] - p[0];
            let dy = cp[1] - p[1];
            let dz = cp[2] - p[2];
            let dist = dx * dx + dy * dy + dz * dz;
            if dist < best_dist {
                best_dist = dist;
                best_poly = Some(idx);
                best_point = cp;
            }
        }

        best_poly.map(|idx| (idx, Vector3::new(best_point[0], best_point[1], best_point[2])))
    }

    /// Find the closest point on a polygon to the given point.
    fn closest_point_on_poly(&self, pverts: &[[f32; 3]], p: [f32; 3]) -> [f32; 3] {
        if Self::point_in_poly_xz(p, pverts) {
            // Point is inside polygon (XZ) — return with polygon's Y
            let y = self.interpolate_height(pverts, p);
            return [p[0], y, p[2]];
        }

        // Point is outside — find closest edge point
        let n = pverts.len();
        let mut min_dist = f32::MAX;
        let mut closest = p;
        for i in 0..n {
            let j = (i + 1) % n;
            let (dist, t) = Self::dist_pt_seg_sq_2d(p, pverts[i], pverts[j]);
            if dist < min_dist {
                min_dist = dist;
                closest = [
                    pverts[i][0] + t * (pverts[j][0] - pverts[i][0]),
                    pverts[i][1] + t * (pverts[j][1] - pverts[i][1]),
                    pverts[i][2] + t * (pverts[j][2] - pverts[i][2]),
                ];
            }
        }
        closest
    }

    /// Interpolate Y height at an XZ point inside a polygon.
    ///
    /// Triangulates the polygon (fan from vertex 0) and uses barycentric
    /// coordinates on the triangle containing the point. Falls back to
    /// closest-vertex height if no triangle contains the point.
    fn interpolate_height(&self, pverts: &[[f32; 3]], p: [f32; 3]) -> f32 {
        // Fan triangulation from vertex 0
        for i in 1..pverts.len() - 1 {
            let a = pverts[0];
            let b = pverts[i];
            let c = pverts[i + 1];

            // Barycentric coordinates on XZ plane
            let v0x = c[0] - a[0]; let v0z = c[2] - a[2];
            let v1x = b[0] - a[0]; let v1z = b[2] - a[2];
            let v2x = p[0] - a[0]; let v2z = p[2] - a[2];

            let dot00 = v0x * v0x + v0z * v0z;
            let dot01 = v0x * v1x + v0z * v1z;
            let dot02 = v0x * v2x + v0z * v2z;
            let dot11 = v1x * v1x + v1z * v1z;
            let dot12 = v1x * v2x + v1z * v2z;

            let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
            let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
            let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

            if u >= -0.01 && v >= -0.01 && (u + v) <= 1.01 {
                return a[1] + u * (c[1] - a[1]) + v * (b[1] - a[1]);
            }
        }

        // Fallback: closest vertex by XZ distance
        let mut best_y = pverts[0][1];
        let mut best_dist = f32::MAX;
        for v in pverts {
            let dx = v[0] - p[0];
            let dz = v[2] - p[2];
            let d = dx * dx + dz * dz;
            if d < best_dist {
                best_dist = d;
                best_y = v[1];
            }
        }
        best_y
    }

    /// Returns `true` if the given position lies on a walkable navmesh polygon.
    pub fn is_point_valid(&self, pos: &Vector3) -> bool {
        // Check if nearest poly is within a reasonable distance
        if let Some((_, closest)) = self.find_nearest_poly(pos) {
            let dist = pos.distance_to(&closest);
            dist < self.agent_radius * 2.0
        } else {
            false
        }
    }

    /// Find the closest valid navmesh position to the given point.
    pub fn get_nearest_point(&self, pos: &Vector3) -> Vector3 {
        self.find_nearest_poly(pos)
            .map(|(_, p)| p)
            .unwrap_or(*pos)
    }

    /// Sample the navmesh surface height at the given XZ position.
    ///
    /// Finds the walkable polygon containing (x, z) and returns the
    /// interpolated Y height on that polygon's surface. Returns `None`
    /// if no walkable polygon contains the point.
    pub fn get_height_at(&self, x: f32, z: f32) -> Option<f32> {
        let p = [x, 0.0, z];
        for (idx, poly) in self.polys.iter().enumerate() {
            if poly.area == 0 { continue; }
            let pverts = self.poly_verts(idx);
            if Self::point_in_poly_xz(p, &pverts) {
                return Some(self.interpolate_height(&pverts, p));
            }
        }
        None
    }

    /// Line-of-sight raycast from `start` to `end`.
    ///
    /// Returns `true` if the ray can travel from start to end without hitting
    /// a navmesh boundary (i.e. there is clear line of sight).
    ///
    /// Algorithm: Start at the polygon containing `start`, test ray against
    /// polygon edges, hop to neighbor through exit edge, repeat until we
    /// reach `end` or hit a boundary (no neighbor on exit edge).
    pub fn raycast(&self, start: &Vector3, end: &Vector3) -> bool {
        let s = [start.x, start.y, start.z];
        let e = [end.x, end.y, end.z];

        // Find starting polygon
        let start_poly = match self.find_nearest_poly(start) {
            Some((idx, closest)) => {
                // Verify start is actually on/near this polygon
                if start.distance_to(&closest) > self.agent_radius * 3.0 {
                    return false;
                }
                idx
            }
            None => return false,
        };

        let mut current_poly = start_poly;
        let max_iters = self.polys.len() * 2; // Safety limit
        // Track the full ray from s to e; we test it per-polygon
        // but advance the current polygon rather than the ray start

        for _ in 0..max_iters {
            let pverts = self.poly_verts(current_poly);

            // Check if the endpoint is inside this polygon
            if Self::point_in_poly_xz(e, &pverts) {
                return true;
            }

            // Find which edge the ray exits through by testing ray against
            // each edge and finding the first exit edge in the ray direction.
            let nv = pverts.len();
            let mut best_exit_t = f32::MAX;
            let mut exit_edge: Option<usize> = None;

            for i in 0..nv {
                let j = (i + 1) % nv;
                let va = pverts[i];
                let vb = pverts[j];

                // Ray-segment intersection in 2D (XZ plane)
                // Ray: P = s + t * (e - s), Edge: Q = va + u * (vb - va)
                let dx = e[0] - s[0];
                let dz = e[2] - s[2];
                let ex = vb[0] - va[0];
                let ez = vb[2] - va[2];

                let denom = dx * ez - dz * ex;
                if denom.abs() < 1e-8 { continue; } // Parallel

                let nx = va[0] - s[0];
                let nz = va[2] - s[2];

                let t = (nx * ez - nz * ex) / denom;
                let u = (nx * dz - nz * dx) / denom;

                // t must be > 0 (forward along ray), u must be in [0, 1] (on edge)
                if t > 1e-6 && t <= 1.0 + 1e-6 && u >= -1e-6 && u <= 1.0 + 1e-6 {
                    if t < best_exit_t {
                        best_exit_t = t;
                        exit_edge = Some(i);
                    }
                }
            }

            let edge = match exit_edge {
                Some(e) => e,
                None => return false, // No exit found — ray stuck
            };

            if best_exit_t >= 1.0 - 1e-6 {
                // Ray reaches endpoint before exiting — clear LoS
                return true;
            }

            // Check for neighbor through exit edge
            if edge >= self.polys[current_poly].neighbors.len() {
                return false;
            }

            let neighbor = self.polys[current_poly].neighbors[edge];
            if neighbor == NO_NEIGHBOR || neighbor as usize >= self.polys.len() {
                return false; // Boundary edge — blocked
            }

            current_poly = neighbor as usize;
        }

        false // Exceeded iteration limit
    }

    /// Find a path from `start` to `end` across the navigation mesh.
    ///
    /// Returns a sequence of world-space waypoints forming a walkable path,
    /// or `None` if no path exists. Uses A* over polygon adjacency followed
    /// by the funnel algorithm to produce a straight path.
    pub fn find_path(&self, start: &Vector3, end: &Vector3) -> Option<Vec<Vector3>> {
        let (start_poly, _) = self.find_nearest_poly(start)?;
        let (end_poly, end_point) = self.find_nearest_poly(end)?;

        if start_poly == end_poly {
            return Some(vec![*start, end_point]);
        }

        // A* over polygon adjacency
        let poly_path = self.astar(start_poly, end_poly)?;

        // Convert polygon corridor to waypoints via funnel algorithm
        let waypoints = self.funnel(start, &end_point, &poly_path);
        Some(waypoints)
    }

    /// A* pathfinding over polygon adjacency graph.
    /// Returns a corridor of polygon indices from start to end.
    fn astar(&self, start: usize, end: usize) -> Option<Vec<usize>> {
        use std::collections::BinaryHeap;
        use std::cmp::Reverse;

        let n = self.polys.len();
        let mut g_cost = vec![f32::MAX; n];
        let mut parent = vec![usize::MAX; n];
        let mut closed = vec![false; n];

        // Priority queue: (Reverse(f_cost_bits), poly_index)
        // Using Reverse for min-heap, f32 bits for ordering
        let mut open: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();

        let start_center = self.poly_center(start);
        let end_center = self.poly_center(end);

        g_cost[start] = 0.0;
        let h = Self::dist_xz(start_center, end_center);
        open.push(Reverse((f32_to_bits_ord(h), start)));

        while let Some(Reverse((_, current))) = open.pop() {
            if current == end {
                // Reconstruct path
                let mut path = Vec::new();
                let mut c = current;
                while c != usize::MAX {
                    path.push(c);
                    c = parent[c];
                }
                path.reverse();
                return Some(path);
            }

            if closed[current] { continue; }
            closed[current] = true;

            let current_center = self.poly_center(current);

            for (edge_idx, &neighbor_idx) in self.polys[current].neighbors.iter().enumerate() {
                if neighbor_idx == NO_NEIGHBOR { continue; }
                let ni = neighbor_idx as usize;
                if ni >= n || closed[ni] { continue; }
                if self.polys[ni].area == 0 { continue; } // Non-walkable

                // Cost = edge midpoint distance
                let edge_mid = self.edge_midpoint(current, edge_idx);
                let step_cost = Self::dist_xz(current_center, edge_mid);
                let new_g = g_cost[current] + step_cost;

                if new_g < g_cost[ni] {
                    g_cost[ni] = new_g;
                    parent[ni] = current;
                    let neighbor_center = self.poly_center(ni);
                    let h = Self::dist_xz(neighbor_center, end_center);
                    let f = new_g + h;
                    open.push(Reverse((f32_to_bits_ord(f), ni)));
                }
            }
        }

        None // No path found
    }

    /// Funnel algorithm: convert polygon corridor to straight-path waypoints.
    ///
    /// Reference: Detour's findStraightPath / Simple Stupid Funnel Algorithm.
    fn funnel(&self, start: &Vector3, end: &Vector3, corridor: &[usize]) -> Vec<Vector3> {
        if corridor.len() <= 1 {
            return vec![*start, *end];
        }

        // Build portal list (left/right edge vertices between adjacent polygons)
        let mut portals: Vec<([f32; 3], [f32; 3])> = Vec::new();
        portals.push(([start.x, start.y, start.z], [start.x, start.y, start.z]));

        for i in 0..corridor.len() - 1 {
            if let Some((left, right)) = self.get_portal_points(corridor[i], corridor[i + 1]) {
                portals.push((left, right));
            }
        }
        portals.push(([end.x, end.y, end.z], [end.x, end.y, end.z]));

        // Simple Stupid Funnel Algorithm
        let mut path = Vec::new();
        path.push(*start);

        let mut apex = [start.x, start.y, start.z];
        let mut left = apex;
        let mut right = apex;
        #[allow(unused_assignments)]
        let mut apex_idx = 0;
        let mut left_idx = 0;
        let mut right_idx = 0;

        let n = portals.len();
        for i in 1..n {
            let (pl, pr) = portals[i];

            // Update right vertex
            if Self::tri_area_2d(apex, right, pr) <= 0.0 {
                if Self::vequal(apex, right) || Self::tri_area_2d(apex, left, pr) > 0.0 {
                    right = pr;
                    right_idx = i;
                } else {
                    // Right over left — insert left as waypoint
                    path.push(Vector3::new(left[0], left[1], left[2]));
                    apex = left;
                    apex_idx = left_idx;
                    right = apex;
                    right_idx = apex_idx;
                    // Restart scan from apex
                    left = apex;
                    left_idx = apex_idx;
                    continue;
                }
            }

            // Update left vertex
            if Self::tri_area_2d(apex, left, pl) >= 0.0 {
                if Self::vequal(apex, left) || Self::tri_area_2d(apex, right, pl) < 0.0 {
                    left = pl;
                    left_idx = i;
                } else {
                    // Left over right — insert right as waypoint
                    path.push(Vector3::new(right[0], right[1], right[2]));
                    apex = right;
                    apex_idx = right_idx;
                    left = apex;
                    left_idx = apex_idx;
                    right = apex;
                    right_idx = apex_idx;
                    continue;
                }
            }
        }

        // Add end point
        let last = path.last().copied().unwrap_or(*start);
        if last != *end {
            path.push(*end);
        }

        path
    }

    /// Get the portal (shared edge) between two adjacent polygons.
    /// Returns (left_vertex, right_vertex) of the shared edge.
    fn get_portal_points(&self, poly_a: usize, poly_b: usize) -> Option<([f32; 3], [f32; 3])> {
        let pa = &self.polys[poly_a];

        // Find which edge of poly_a connects to poly_b
        for (edge, &neighbor) in pa.neighbors.iter().enumerate() {
            if neighbor as usize == poly_b {
                let nv = pa.vert_indices.len();
                let vi0 = pa.vert_indices[edge] as usize;
                let vi1 = pa.vert_indices[(edge + 1) % nv] as usize;
                return Some((self.verts[vi0], self.verts[vi1]));
            }
        }
        None
    }

    /// Center of a polygon (average of vertices) on XZ plane.
    fn poly_center(&self, poly_idx: usize) -> [f32; 3] {
        let pverts = self.poly_verts(poly_idx);
        let n = pverts.len() as f32;
        let mut c = [0.0f32; 3];
        for v in &pverts {
            c[0] += v[0];
            c[1] += v[1];
            c[2] += v[2];
        }
        [c[0] / n, c[1] / n, c[2] / n]
    }

    /// Midpoint of a polygon edge.
    fn edge_midpoint(&self, poly_idx: usize, edge: usize) -> [f32; 3] {
        let pa = &self.polys[poly_idx];
        let nv = pa.vert_indices.len();
        let v0 = self.verts[pa.vert_indices[edge] as usize];
        let v1 = self.verts[pa.vert_indices[(edge + 1) % nv] as usize];
        [
            (v0[0] + v1[0]) * 0.5,
            (v0[1] + v1[1]) * 0.5,
            (v0[2] + v1[2]) * 0.5,
        ]
    }

    /// 2D distance on XZ plane.
    fn dist_xz(a: [f32; 3], b: [f32; 3]) -> f32 {
        let dx = a[0] - b[0];
        let dz = a[2] - b[2];
        (dx * dx + dz * dz).sqrt()
    }

    /// Check if two points are approximately equal.
    fn vequal(a: [f32; 3], b: [f32; 3]) -> bool {
        const EPS: f32 = 0.001 * 0.001;
        let dx = a[0] - b[0];
        let dz = a[2] - b[2];
        (dx * dx + dz * dz) < EPS
    }
}

/// Convert f32 to a u32 that preserves ordering for use in BinaryHeap.
fn f32_to_bits_ord(f: f32) -> u32 {
    let bits = f.to_bits();
    if bits & 0x8000_0000 != 0 {
        !bits // Negative: flip all bits
    } else {
        bits ^ 0x8000_0000 // Positive: flip sign bit
    }
}

impl std::fmt::Debug for NavMesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavMesh")
            .field("name", &self.name)
            .field("polys", &self.polys.len())
            .field("verts", &self.verts.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a simple 2-polygon navmesh for testing:
    ///
    /// ```text
    ///   (0,0)──(5,0)──(10,0)
    ///     │   poly 0  │  poly 1  │
    ///   (0,5)──(5,5)──(10,5)
    /// ```
    fn test_mesh() -> NavMesh {
        NavMesh {
            name: "test".to_string(),
            verts: vec![
                [0.0, 0.0, 0.0],  // 0
                [5.0, 0.0, 0.0],  // 1
                [5.0, 0.0, 5.0],  // 2
                [0.0, 0.0, 5.0],  // 3
                [10.0, 0.0, 0.0], // 4
                [10.0, 0.0, 5.0], // 5
            ],
            polys: vec![
                Poly {
                    vert_indices: vec![0, 1, 2, 3],
                    neighbors: vec![NO_NEIGHBOR, 1, NO_NEIGHBOR, NO_NEIGHBOR],
                    area: 1,
                },
                Poly {
                    vert_indices: vec![1, 4, 5, 2],
                    neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR, 0],
                    area: 1,
                },
            ],
            nvp: 6,
            agent_height: 2.0,
            agent_radius: 0.6,
            bmin: [0.0, 0.0, 0.0],
            bmax: [10.0, 0.0, 5.0],
        }
    }

    #[test]
    fn find_nearest_poly_center() {
        let mesh = test_mesh();
        let (idx, _) = mesh.find_nearest_poly(&Vector3::new(2.5, 0.0, 2.5)).unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn find_nearest_poly_second() {
        let mesh = test_mesh();
        let (idx, _) = mesh.find_nearest_poly(&Vector3::new(7.5, 0.0, 2.5)).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn raycast_same_poly() {
        let mesh = test_mesh();
        assert!(mesh.raycast(
            &Vector3::new(1.0, 0.0, 1.0),
            &Vector3::new(4.0, 0.0, 4.0),
        ));
    }

    #[test]
    fn raycast_across_polys() {
        let mesh = test_mesh();
        assert!(mesh.raycast(
            &Vector3::new(2.5, 0.0, 2.5),
            &Vector3::new(7.5, 0.0, 2.5),
        ));
    }

    #[test]
    fn raycast_blocked_by_boundary() {
        let mesh = test_mesh();
        // Ray from inside poly 0 to outside the mesh entirely
        assert!(!mesh.raycast(
            &Vector3::new(2.5, 0.0, 2.5),
            &Vector3::new(-5.0, 0.0, 2.5),
        ));
    }

    #[test]
    fn pathfind_across_polys() {
        let mesh = test_mesh();
        let path = mesh.find_path(
            &Vector3::new(2.5, 0.0, 2.5),
            &Vector3::new(7.5, 0.0, 2.5),
        );
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.len() >= 2);
        // Start and end should match
        assert!((path[0].x - 2.5).abs() < 0.01);
        assert!((path.last().unwrap().x - 7.5).abs() < 0.01);
    }

    #[test]
    fn point_valid_on_mesh() {
        let mesh = test_mesh();
        assert!(mesh.is_point_valid(&Vector3::new(2.5, 0.0, 2.5)));
    }

    #[test]
    fn point_invalid_off_mesh() {
        let mesh = test_mesh();
        assert!(!mesh.is_point_valid(&Vector3::new(100.0, 0.0, 100.0)));
    }

    #[test]
    fn load_castle_cellblock_nav() {
        let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
        if !path.exists() {
            // Skip test if data file not present (CI)
            return;
        }
        let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");
        assert!(mesh.polys.len() > 0);
        assert!(mesh.verts.len() > 0);
        // The guard spawn position should be on the navmesh
        let guard_pos = Vector3::new(-289.465, 68.542, -154.276);
        assert!(mesh.is_point_valid(&guard_pos),
            "Guard spawn position should be on navmesh");
    }
}

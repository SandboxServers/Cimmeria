//! Connectivity analysis for a parsed XRC `.nav` polygon mesh.
//!
//! A `.nav` that loads cleanly is not necessarily a *usable* navmesh: a
//! build can silently produce two islands separated by a one-cell gap, and
//! an NPC on the wrong island will never path to a player on the other.
//! CA14's acceptance criterion is "one connected walkable region between
//! the named probe points", so this module answers exactly that question
//! without going through Detour.
//!
//! # Polygon layout
//!
//! The `polys` array from [`crate::nav_roundtrip::XrcNav`] is Recast's
//! `rcPolyMesh::polys`, `nvp * 2` `u16`s per polygon:
//!
//! ```text
//! [0 .. nvp)        vertex indices, terminated early by 0xffff
//! [nvp .. nvp*2)    per-edge neighbour polygon index, 0xffff == none
//! ```
//!
//! The neighbour slot stores the adjacent polygon index **directly** (not
//! `index + 1` as Detour's internal representation does) — see
//! `RecastMesh.cpp::buildMeshAdjacency`, which memsets the whole array to
//! `0xff` and then writes `p0[nvp + edge] = otherPoly`. When
//! `mesh.borderSize > 0` Recast repurposes the slot's high bit
//! (`0x8000`) as a tile-portal marker; every shipped SGW `.nav` has
//! `border_size == 0`, but we mask defensively and treat a portal edge as
//! "no in-tile neighbour" rather than mis-decoding it as poly `0x7fff`.
//!
//! Connectivity here therefore matches what Detour will see at runtime:
//! `dtCreateNavMeshData` copies the same per-edge links into the tile.
//!
//! # Coordinates
//!
//! Vertices are quantised `u16` grid coordinates. World (BigWorld) space is
//! `bmin + (x * cs, y * ch, z * cs)` — the horizontal axes use the cell
//! size, the vertical axis the cell height. Same reconstruction as
//! `crates/entity/src/navigation/mod.rs` hands to Detour.

pub mod gaps;
#[cfg(test)]
pub(crate) mod test_mesh;
mod tiled;

pub use gaps::{Approach, BoundaryEdge, GapGraph};

use crate::nav_roundtrip::XrcNav;

/// Recast's "no index" sentinel, used both as the vertex-list terminator
/// and as the "no neighbour" marker in the adjacency half of a poly.
const RC_MESH_NULL_IDX: u16 = 0xffff;

/// High bit set on a neighbour slot means "external tile portal", not a
/// polygon index. Only produced when `border_size > 0`.
const RC_PORTAL_FLAG: u16 = 0x8000;

/// One decoded polygon.
#[derive(Debug, Clone)]
pub struct NavPoly {
    /// Indices into [`NavGraph::verts`], in winding order.
    pub verts: Vec<u32>,
    /// Per-edge neighbour polygon index. `edge i` joins `verts[i]` to
    /// `verts[(i + 1) % n]`. `None` == boundary edge or tile portal.
    pub neighbours: Vec<Option<u32>>,
    /// Polygons in *other* tiles this one is linked to across a tile
    /// portal (tiled `.nav` only; see [`NavGraph::from_tiled`]). Portal edges keep `None`
    /// in [`Self::neighbours`] because one portal edge can link to several
    /// polygons on the other side.
    pub portal_links: Vec<u32>,
    pub area: u8,
    pub flags: u16,
    pub region: u16,
}

/// Per-component roll-up.
#[derive(Debug, Clone)]
pub struct ComponentStat {
    pub id: u32,
    pub poly_count: usize,
    /// Sum of XZ-projected polygon areas, in BigWorld units squared.
    pub area_xz: f64,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
}

/// Result of locating a probe point against the mesh.
#[derive(Debug, Clone)]
pub struct ProbeHit {
    pub poly: u32,
    pub component: u32,
    /// Horizontal (XZ) distance from the probe to the polygon. Zero when
    /// the probe is inside the polygon's XZ footprint.
    pub horizontal_distance: f32,
    /// Signed `probe.y - surface.y`. Positive means the probe floats above
    /// the mesh.
    pub vertical_distance: f32,
    /// Closest point on the polygon surface.
    pub closest: [f32; 3],
}

/// A `.nav` decoded into world-space vertices, polygons, adjacency and
/// connected components.
#[derive(Debug, Clone)]
pub struct NavGraph {
    pub cs: f32,
    pub ch: f32,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
    pub verts: Vec<[f32; 3]>,
    pub polys: Vec<NavPoly>,
    /// Component id per polygon, parallel to [`Self::polys`].
    pub component: Vec<u32>,
    pub component_count: u32,
    /// Edges where `a` names `b` as a neighbour but `b` does not name `a`.
    /// Recast always writes both halves, so a non-empty list means the file
    /// was hand-edited or we mis-decoded the layout — surfaced rather than
    /// swallowed.
    pub asymmetric_links: Vec<(u32, u32)>,
}

impl NavGraph {
    /// Decode a parsed `.nav` and run the flood fill.
    pub fn from_nav(nav: &XrcNav) -> Self {
        let verts = (0..nav.nverts as usize)
            .map(|i| {
                [
                    nav.bmin[0] + nav.verts[i * 3] as f32 * nav.cs,
                    nav.bmin[1] + nav.verts[i * 3 + 1] as f32 * nav.ch,
                    nav.bmin[2] + nav.verts[i * 3 + 2] as f32 * nav.cs,
                ]
            })
            .collect();

        let nvp = nav.nvp as usize;
        let mut polys = Vec::with_capacity(nav.npolys as usize);
        for p in 0..nav.npolys as usize {
            let base = p * nvp * 2;
            let mut indices = Vec::with_capacity(nvp);
            for j in 0..nvp {
                let v = nav.polys[base + j];
                if v == RC_MESH_NULL_IDX {
                    break;
                }
                indices.push(v as u32);
            }
            let n = indices.len();
            let neighbours = (0..n)
                .map(|j| {
                    let nei = nav.polys[base + nvp + j];
                    if nei == RC_MESH_NULL_IDX || nei & RC_PORTAL_FLAG != 0 {
                        None
                    } else {
                        Some(nei as u32)
                    }
                })
                .collect();
            polys.push(NavPoly {
                verts: indices,
                neighbours,
                portal_links: Vec::new(),
                area: nav.areas[p],
                flags: nav.flags[p],
                region: nav.regs[p],
            });
        }

        let mut graph = Self {
            cs: nav.cs,
            ch: nav.ch,
            bmin: nav.bmin,
            bmax: nav.bmax,
            verts,
            polys,
            component: Vec::new(),
            component_count: 0,
            asymmetric_links: Vec::new(),
        };
        graph.check_symmetry();
        graph.flood_fill();
        graph
    }

    fn check_symmetry(&mut self) {
        for (a, poly) in self.polys.iter().enumerate() {
            for nei in poly.neighbours.iter().flatten() {
                let b = *nei as usize;
                let mutual = self
                    .polys
                    .get(b)
                    .map(|q| q.neighbours.iter().flatten().any(|x| *x as usize == a))
                    .unwrap_or(false);
                if !mutual {
                    self.asymmetric_links.push((a as u32, *nei));
                }
            }
        }
    }

    /// Iterative (not recursive — a 19k-poly mesh would blow the stack)
    /// flood fill over the adjacency graph.
    fn flood_fill(&mut self) {
        const UNVISITED: u32 = u32::MAX;
        let mut comp = vec![UNVISITED; self.polys.len()];
        let mut next_id = 0u32;
        let mut stack: Vec<u32> = Vec::new();

        for seed in 0..self.polys.len() {
            if comp[seed] != UNVISITED {
                continue;
            }
            let id = next_id;
            next_id += 1;
            comp[seed] = id;
            stack.push(seed as u32);
            while let Some(cur) = stack.pop() {
                // Collect first to keep the borrow of `self.polys` short.
                let poly = &self.polys[cur as usize];
                let neighbours: Vec<u32> = poly
                    .neighbours
                    .iter()
                    .flatten()
                    .chain(poly.portal_links.iter())
                    .copied()
                    .collect();
                for nei in neighbours {
                    let n = nei as usize;
                    if n < comp.len() && comp[n] == UNVISITED {
                        comp[n] = id;
                        stack.push(nei);
                    }
                }
            }
        }

        self.component = comp;
        self.component_count = next_id;
    }

    /// Per-component polygon count, XZ area and bounds, sorted by
    /// descending area so the "main" region is always index 0.
    pub fn component_stats(&self) -> Vec<ComponentStat> {
        let mut stats: Vec<ComponentStat> = (0..self.component_count)
            .map(|id| ComponentStat {
                id,
                poly_count: 0,
                area_xz: 0.0,
                bmin: [f32::INFINITY; 3],
                bmax: [f32::NEG_INFINITY; 3],
            })
            .collect();

        for (p, poly) in self.polys.iter().enumerate() {
            let s = &mut stats[self.component[p] as usize];
            s.poly_count += 1;
            s.area_xz += self.poly_area_xz(poly);
            for vi in &poly.verts {
                let v = self.verts[*vi as usize];
                for (k, c) in v.iter().enumerate() {
                    s.bmin[k] = s.bmin[k].min(*c);
                    s.bmax[k] = s.bmax[k].max(*c);
                }
            }
        }
        stats.sort_by(|a, b| b.area_xz.total_cmp(&a.area_xz));
        stats
    }

    /// Shoelace area of the polygon projected onto XZ.
    fn poly_area_xz(&self, poly: &NavPoly) -> f64 {
        let n = poly.verts.len();
        if n < 3 {
            return 0.0;
        }
        let mut acc = 0.0f64;
        for i in 0..n {
            let a = self.verts[poly.verts[i] as usize];
            let b = self.verts[poly.verts[(i + 1) % n] as usize];
            acc += a[0] as f64 * b[2] as f64 - b[0] as f64 * a[2] as f64;
        }
        (acc * 0.5).abs()
    }

    /// Locate `p` on the mesh, **preferring polygons that are actually
    /// within tolerance**.
    ///
    /// This is the one callers that have tolerances should use.
    /// [`Self::locate`] answers "which polygon is nearest in XZ", which
    /// is a different question and gives the wrong answer on a stacked
    /// mesh: a buried sheet whose footprint covers the probe wins over a
    /// real floor half a metre to the side, and the probe is then
    /// reported out of tolerance although it is standing on the mesh.
    /// Measured on the 17-tile Castle interior build, the BSP hull skin
    /// at BW y ~79.5 did exactly that to `throne_room` (dy -41.35),
    /// `opcore` (dy -9.37) and `armory`.
    ///
    /// Among polygons within both tolerances the winner is the one
    /// closest in 3-D (`hypot(horizontal, vertical)`) — not
    /// horizontal-first, because a probe sitting in a 0.3 m gap between
    /// two floor polygons should resolve to the floor beside it rather
    /// than to whatever happens to be directly below.
    ///
    /// Falls back to [`Self::locate`] when nothing qualifies, so an
    /// off-mesh probe still reports how far off it is.
    pub fn locate_within(&self, p: [f32; 3], h_tol: f32, v_tol: f32) -> Option<ProbeHit> {
        let mut best: Option<(f32, ProbeHit)> = None;
        for (idx, poly) in self.polys.iter().enumerate() {
            let hit = self.closest_on_poly(idx as u32, poly, p);
            if hit.horizontal_distance > h_tol || hit.vertical_distance.abs() > v_tol {
                continue;
            }
            let d = hit.horizontal_distance.hypot(hit.vertical_distance).abs();
            if best.as_ref().map(|(bd, _)| d < *bd).unwrap_or(true) {
                best = Some((d, hit));
            }
        }
        best.map(|(_, h)| h).or_else(|| self.locate(p))
    }

    /// Locate `p` on the mesh, ignoring tolerances.
    ///
    /// Prefers a polygon whose XZ footprint contains `p` (picking the one
    /// with the smallest vertical distance, so stacked floors resolve to
    /// the right storey). Falls back to the polygon with the smallest
    /// horizontal distance when the probe is off-mesh.
    ///
    /// Callers that have tolerances want [`Self::locate_within`] — see
    /// there for why.
    pub fn locate(&self, p: [f32; 3]) -> Option<ProbeHit> {
        let mut inside: Option<ProbeHit> = None;
        let mut outside: Option<ProbeHit> = None;

        for (idx, poly) in self.polys.iter().enumerate() {
            let hit = self.closest_on_poly(idx as u32, poly, p);
            let slot = if hit.horizontal_distance <= 0.0 {
                &mut inside
            } else {
                &mut outside
            };
            let better = match slot.as_ref() {
                None => true,
                Some(cur) => {
                    if hit.horizontal_distance < cur.horizontal_distance {
                        true
                    } else if hit.horizontal_distance > cur.horizontal_distance {
                        false
                    } else {
                        hit.vertical_distance.abs() < cur.vertical_distance.abs()
                    }
                }
            };
            if better {
                *slot = Some(hit);
            }
        }
        inside.or(outside)
    }

    fn closest_on_poly(&self, idx: u32, poly: &NavPoly, p: [f32; 3]) -> ProbeHit {
        let n = poly.verts.len();
        // Fan-triangulate from vertex 0 — Recast polygons are convex, so a
        // fan is a valid decomposition and the barycentric interpolation
        // below gives the exact surface height.
        for i in 1..n.saturating_sub(1) {
            let a = self.verts[poly.verts[0] as usize];
            let b = self.verts[poly.verts[i] as usize];
            let c = self.verts[poly.verts[i + 1] as usize];
            if let Some(y) = barycentric_height(a, b, c, p[0], p[2]) {
                return ProbeHit {
                    poly: idx,
                    component: self.component[idx as usize],
                    horizontal_distance: 0.0,
                    vertical_distance: p[1] - y,
                    closest: [p[0], y, p[2]],
                };
            }
        }

        // Off the footprint — nearest point on the boundary.
        let mut best_d2 = f32::INFINITY;
        let mut best = [0.0f32; 3];
        for i in 0..n {
            let a = self.verts[poly.verts[i] as usize];
            let b = self.verts[poly.verts[(i + 1) % n] as usize];
            let q = closest_on_segment_xz(a, b, p[0], p[2]);
            let d2 = (q[0] - p[0]).powi(2) + (q[2] - p[2]).powi(2);
            if d2 < best_d2 {
                best_d2 = d2;
                best = q;
            }
        }
        ProbeHit {
            poly: idx,
            component: self.component[idx as usize],
            horizontal_distance: best_d2.sqrt(),
            vertical_distance: p[1] - best[1],
            closest: best,
        }
    }
}

/// Barycentric height of `(x, z)` inside triangle `a,b,c` projected on XZ.
/// Returns `None` when the point is outside the triangle.
fn barycentric_height(a: [f32; 3], b: [f32; 3], c: [f32; 3], x: f32, z: f32) -> Option<f32> {
    let denom = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
    if denom.abs() < 1e-9 {
        return None; // degenerate in projection
    }
    let l1 = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / denom;
    let l2 = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / denom;
    let l3 = 1.0 - l1 - l2;
    // Small negative tolerance so a probe exactly on a shared fan edge
    // resolves instead of falling through to the boundary path.
    const EPS: f32 = -1e-4;
    if l1 < EPS || l2 < EPS || l3 < EPS {
        return None;
    }
    Some(l1 * a[1] + l2 * b[1] + l3 * c[1])
}

/// Closest point on segment `a→b` to `(x, z)`, measured in XZ; the returned
/// point carries the interpolated height.
fn closest_on_segment_xz(a: [f32; 3], b: [f32; 3], x: f32, z: f32) -> [f32; 3] {
    let dx = b[0] - a[0];
    let dz = b[2] - a[2];
    let len2 = dx * dx + dz * dz;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((x - a[0]) * dx + (z - a[2]) * dz) / len2).clamp(0.0, 1.0)
    };
    [a[0] + dx * t, a[1] + (b[1] - a[1]) * t, a[2] + dz * t]
}

#[cfg(test)]
mod tests;

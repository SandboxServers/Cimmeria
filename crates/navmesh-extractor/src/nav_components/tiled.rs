//! [`NavGraph`] for a tiled (`XRCT`) `.nav`: every tile's polygons in one
//! graph, with the tile-portal edges linked the way Detour links them.
//!
//! Inside a tile the neighbour slots are polygon indices, as in a
//! single-mesh file. On a tile's sides they hold Recast's portal marker,
//! `0x8000 | dir` (`RecastMesh.cpp`): `0` = the tile's min-X side, `1` =
//! max-Z, `2` = max-X, `3` = min-Z. At load time `dtNavMesh::connectExtLinks`
//! pairs each portal edge with the edges on the facing side of the
//! neighbouring tile that
//!
//! - lie on the same line (`findConnectingPolys`: within 0.01 m), and
//! - overlap along it, shrunk by 0.01 m at each end so edges that only
//!   touch at a corner do not link, with the two edges' heights crossing or
//!   within `2 * walkableClimb` at an end of the overlap
//!   (`overlapSlabs`).
//!
//! [`NavGraph::from_tiled`] applies exactly that test, so the component
//! count `nav_inspect` reports for a tiled file is the one the server's
//! Detour mesh has.

use std::collections::HashMap;

use super::{NavGraph, NavPoly, RC_MESH_NULL_IDX, RC_PORTAL_FLAG};
use crate::nav_tiled::XrcTiledNav;

/// `findConnectingPolys`' same-line tolerance and `overlapSlabs`' end
/// shrink, both 0.01 in Detour.
const SLAB_EPS: f32 = 0.01;

/// One portal edge, in world coordinates.
struct PortalEdge {
    poly: u32,
    a: [f32; 3],
    b: [f32; 3],
}

impl PortalEdge {
    /// The coordinate that is constant along the edge (X for sides 0 / 2,
    /// Z for 1 / 3) and the slab: `(u, y)` at both ends, sorted by `u`,
    /// where `u` runs along the edge.
    fn slab(&self, dir: u16) -> (f32, [f32; 2], [f32; 2]) {
        let (line, ua, ub) = if dir == 0 || dir == 2 {
            (self.a[0], self.a[2], self.b[2])
        } else {
            (self.a[2], self.a[0], self.b[0])
        };
        let (pa, pb) = ([ua, self.a[1]], [ub, self.b[1]]);
        if ua <= ub {
            (line, pa, pb)
        } else {
            (line, pb, pa)
        }
    }
}

/// Detour's `overlapSlabs` (DetourNavMesh.cpp), with `px = 0.01` and
/// `py = walkableClimb`.
fn overlap_slabs(amin: [f32; 2], amax: [f32; 2], bmin: [f32; 2], bmax: [f32; 2], py: f32) -> bool {
    let minx = (amin[0] + SLAB_EPS).max(bmin[0] + SLAB_EPS);
    let maxx = (amax[0] - SLAB_EPS).min(bmax[0] - SLAB_EPS);
    if minx > maxx {
        return false;
    }
    let ad = (amax[1] - amin[1]) / (amax[0] - amin[0]);
    let ak = amin[1] - ad * amin[0];
    let bd = (bmax[1] - bmin[1]) / (bmax[0] - bmin[0]);
    let bk = bmin[1] - bd * bmin[0];
    let dmin = (bd * minx + bk) - (ad * minx + ak);
    let dmax = (bd * maxx + bk) - (ad * maxx + ak);
    if dmin * dmax < 0.0 {
        return true;
    }
    let thr = (py * 2.0) * (py * 2.0);
    dmin * dmin <= thr || dmax * dmax <= thr
}

/// The side facing `dir` on the neighbouring tile, and that tile's offset.
fn facing(dir: u16) -> (u16, i32, i32) {
    match dir {
        0 => (2, -1, 0),
        1 => (3, 0, 1),
        2 => (0, 1, 0),
        _ => (1, 0, -1),
    }
}

impl NavGraph {
    /// Decode a tiled `.nav` into one graph: tile-local vertex and polygon
    /// indices offset into global ones, portal edges linked across tile
    /// borders, then the same symmetry check and flood fill as
    /// [`Self::from_nav`].
    pub fn from_tiled(nav: &XrcTiledNav) -> Self {
        let mut verts: Vec<[f32; 3]> = Vec::new();
        let mut polys: Vec<NavPoly> = Vec::new();
        let mut bmin = [f32::INFINITY; 3];
        let mut bmax = [f32::NEG_INFINITY; 3];
        // (tile_x, tile_y, side) -> portal edges on that side.
        let mut portals: HashMap<(i32, i32, u16), Vec<PortalEdge>> = HashMap::new();

        for tile in &nav.tiles {
            let m = &tile.mesh;
            let vbase = verts.len() as u32;
            let pbase = polys.len() as u32;
            for k in 0..3 {
                bmin[k] = bmin[k].min(m.bmin[k]);
                bmax[k] = bmax[k].max(m.bmax[k]);
            }
            for i in 0..m.nverts as usize {
                verts.push([
                    m.bmin[0] + m.verts[i * 3] as f32 * m.cs,
                    m.bmin[1] + m.verts[i * 3 + 1] as f32 * m.ch,
                    m.bmin[2] + m.verts[i * 3 + 2] as f32 * m.cs,
                ]);
            }

            let nvp = m.nvp as usize;
            for p in 0..m.npolys as usize {
                let base = p * nvp * 2;
                let local: Vec<u16> = m.polys[base..base + nvp]
                    .iter()
                    .copied()
                    .take_while(|v| *v != RC_MESH_NULL_IDX)
                    .collect();
                let n = local.len();
                let mut neighbours = Vec::with_capacity(n);
                for j in 0..n {
                    let nei = m.polys[base + nvp + j];
                    if nei == RC_MESH_NULL_IDX {
                        neighbours.push(None);
                    } else if nei & RC_PORTAL_FLAG != 0 {
                        neighbours.push(None);
                        let side = nei & 0xf;
                        if side <= 3 {
                            let a = verts[(vbase + local[j] as u32) as usize];
                            let b = verts[(vbase + local[(j + 1) % n] as u32) as usize];
                            portals
                                .entry((tile.tile_x, tile.tile_y, side))
                                .or_default()
                                .push(PortalEdge {
                                    poly: pbase + p as u32,
                                    a,
                                    b,
                                });
                        }
                    } else {
                        neighbours.push(Some(pbase + nei as u32));
                    }
                }
                polys.push(NavPoly {
                    verts: local.iter().map(|v| vbase + *v as u32).collect(),
                    neighbours,
                    portal_links: Vec::new(),
                    area: m.areas[p],
                    flags: m.flags[p],
                    region: m.regs[p],
                });
            }
        }

        for (&(tx, ty, side), edges) in &portals {
            let (other_side, dx, dy) = facing(side);
            let Some(others) = portals.get(&(tx + dx, ty + dy, other_side)) else {
                continue;
            };
            for e in edges {
                let (line, amin, amax) = e.slab(side);
                for o in others {
                    let (oline, bmin_s, bmax_s) = o.slab(other_side);
                    if (line - oline).abs() > SLAB_EPS {
                        continue;
                    }
                    if overlap_slabs(amin, amax, bmin_s, bmax_s, nav.agent_climb) {
                        polys[e.poly as usize].portal_links.push(o.poly);
                    }
                }
            }
        }

        let first = nav.tiles.first().map(|t| (t.mesh.cs, t.mesh.ch));
        let (cs, ch) = first.unwrap_or((0.0, 0.0));
        let mut graph = Self {
            cs,
            ch,
            bmin,
            bmax,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav_components::test_mesh::MeshFixture;
    use crate::nav_tiled::XrcTile;

    /// A tile holding one 4 x 4 quad at grid height `y`, whose min-X and
    /// max-X edges are portals. MeshFixture's quad goes (x,z), (x+w,z),
    /// (x+w,z+d), (x,z+d), so edge 1 is max-X and edge 3 is min-X.
    fn tile(tile_x: i32, y: u16) -> XrcTile {
        let mut f = MeshFixture::new();
        let q = f.wide_quad(0, y, 0, 4, 4);
        f.polys[q as usize][4 + 1] = RC_PORTAL_FLAG | 2;
        f.polys[q as usize][4 + 3] = RC_PORTAL_FLAG;
        let mut mesh = f.build();
        mesh.bmin = [tile_x as f32 * 4.0, 0.0, 0.0];
        mesh.bmax = [tile_x as f32 * 4.0 + 4.0, 10.0, 4.0];
        XrcTile {
            tile_x,
            tile_y: 0,
            mesh,
        }
    }

    fn file(tiles: Vec<XrcTile>) -> XrcTiledNav {
        XrcTiledNav {
            agent_height: 1.8,
            agent_climb: 0.6,
            agent_radius: 0.6,
            orig: [0.0; 3],
            tile_width: 4.0,
            tile_height: 4.0,
            max_tile_polys: 1,
            tiles,
        }
    }

    #[test]
    fn portal_edges_join_tiles_into_one_component() {
        let g = NavGraph::from_tiled(&file(vec![tile(0, 0), tile(1, 0), tile(2, 0)]));
        assert_eq!(g.polys.len(), 3);
        assert_eq!(g.component_count, 1);
        assert!(g.asymmetric_links.is_empty());
        assert_eq!(g.polys[1].portal_links.len(), 2);
    }

    /// The height half of the link test: at climb 0.6 Detour links edges
    /// within `2 * 0.6 = 1.2` m of each other, so a 2 m step stays split
    /// and a 1 m step (below) joins.
    #[test]
    fn a_step_past_twice_the_climb_does_not_link() {
        let g = NavGraph::from_tiled(&file(vec![tile(0, 0), tile(1, 2)]));
        assert_eq!(g.component_count, 2);
    }

    #[test]
    fn a_step_within_twice_the_climb_links() {
        let g = NavGraph::from_tiled(&file(vec![tile(0, 0), tile(1, 1)]));
        assert_eq!(g.component_count, 1);
    }

    #[test]
    fn tiles_that_are_not_adjacent_do_not_link() {
        let g = NavGraph::from_tiled(&file(vec![tile(0, 0), tile(2, 0)]));
        assert_eq!(g.component_count, 2);
    }

    #[test]
    fn probes_resolve_in_either_tile() {
        let g = NavGraph::from_tiled(&file(vec![tile(0, 0), tile(1, 0)]));
        let hit = g.locate([6.0, 0.0, 2.0]).unwrap();
        assert_eq!(hit.horizontal_distance, 0.0);
        assert_eq!(hit.poly, 1);
    }
}

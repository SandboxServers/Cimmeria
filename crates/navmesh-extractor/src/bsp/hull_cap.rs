//! Outer-hull cap detection — the rule that keeps the Castle interior's
//! buried CSG skin out of the navmesh.
//!
//! # What a hull cap is
//!
//! The Castle interior is authored as a stack of large **additive**
//! brushes forming one solid block per chunk, with the rooms carved out
//! of it subtractively. CSG leaves the block's outer skin in the BSP:
//! a horizontal top face at BW y 79.36 and a bottom face at BW y 26.88
//! in every one of the 13 chunks that carry the hull, each covering
//! most of the chunk's 100 x 100 m footprint.
//!
//! Those two planes are 122,801 m² (top) and 123,394 m² (bottom) of the
//! ~361,000 m² of near-horizontal BSP surface in `Maps/Castle` — 68 %
//! of it. The top plane is *upward*-facing, so Recast rasterises it as
//! walkable. On the whole-map build it is 87,709 m² of walkable surface
//! (8.1 % of the mesh) that no player can reach: sealed under terrain
//! at BW y 93-204 with no opening.
//!
//! Two things it is **not**:
//!
//! - It is not why `throne_room` and `opcore` read as missing floors.
//!   That was `NavGraph::locate` preferring any polygon whose XZ
//!   footprint contained the probe over one within tolerance beside it;
//!   fixed in `nav_components::locate_within`, after which both probes
//!   land on their floor with the skin still present.
//! - It is not what pushes a whole-map build past Recast's polygon
//!   budget. Measured on the 144-chunk build: dropping the skin moves
//!   the mesh from 45,115 verts / 21,799 polys to 45,200 / 21,801 — it
//!   costs 85 vertices rather than saving any, because a single huge
//!   flat sheet is cheap in polygons and removing it lets the geometry
//!   underneath contour separately.
//!
//! What it buys is a mesh whose walkable set is reachable: nothing can
//! snap an NPC onto a shelf 41 m above the room it is standing in.
//!
//! # The rule
//!
//! A face is hull skin when **both** of these hold:
//!
//! 1. **It is on the model's outer surface.** Over the triangles that
//!    survived the `PolyFlags` filter, take the Z extent
//!    `[z_min, z_max]` of the *emitted faces* (**not** `Model::points`,
//!    which carries unreferenced entries up at z = 270,000 cm and would
//!    put `z_max` 2.6 km in the air). An up-facing near-horizontal face
//!    on `z_max`, or a down-facing one on `z_min`, has no geometry of
//!    its own model beyond it.
//! 2. **It is buried.** The chunk's own terrain surface lies above the
//!    face at every one of its vertices — see [`TerrainCeiling`]. A
//!    face with no terrain over it, or with terrain *below* it, is kept.
//!
//! Condition 2 is not decoration, it is what makes the rule safe.
//! Condition 1 alone is a plausible-sounding geometric heuristic and it
//! is **wrong**: run on `Maps/Castle_CellBlock` it removes the 27,354 m²
//! sheet at BW y 94.6 and the 17,822 m² sheet at BW y 53.3 — two of the
//! three large flat sheets the shipped `data/spaces/castle_cellblock.nav`
//! actually contains. There the interior is *above* its terrain (a flat
//! plane at BW y 0), so nothing is buried and nothing is dropped. In
//! Castle the interior is 45 m under the terrain and the skin goes.
//!
//! Terrain holes (`TID_Visibility_Off`) leave no triangles, so a face
//! under an opening has no ceiling sample and is kept. That is the
//! behaviour we want: an opening is exactly how a player would reach it.
//!
//! Up/down is decided from the **authored** surface normal
//! (`Vectors[vNormal]`), never from the emitted winding: winding is a
//! property of the node vertex pool plus [`super::EMIT_REVERSED`], and
//! the classifier must not change meaning if that constant does.

/// Vertical distance, in UE3 cm, within which a vertex counts as lying
/// *on* an extreme plane. UE3 brush corners snap to integer
/// centimetres, so 1 cm is generous.
pub const CAP_PLANE_EPSILON_CM: f32 = 1.0;

/// `|n.z|` above which an authored surface normal counts as
/// near-horizontal (~30 degrees from flat). Matches the threshold the
/// BSP analysis tests bucket with, so the measured areas and the
/// filtered areas describe the same set of faces.
pub const NEAR_HORIZONTAL_NZ: f32 = 0.86;

/// Edge length, in UE3 cm, of one [`TerrainCeiling`] sample cell.
///
/// Castle terrain is authored at 100 cm per patch vertex, so a 200 cm
/// cell always contains real samples and the grid over a 100 m chunk is
/// 50 x 50.
pub const CEILING_CELL_CM: f32 = 200.0;

/// Hard cap on a [`TerrainCeiling`] grid edge, so a terrain actor with
/// a wild vertex can't ask for a gigabyte of buckets.
const CEILING_MAX_CELLS: usize = 1_024;

/// The two extreme horizontal planes of one `Model`'s emitted faces.
///
/// Construct with [`HullCap::detect`]; `None` means the model is too
/// shallow to have a hull and nothing should be dropped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HullCap {
    /// Lowest emitted-face Z, UE3 cm.
    pub z_min: f32,
    /// Highest emitted-face Z, UE3 cm.
    pub z_max: f32,
}

impl HullCap {
    /// Find the outer planes of a model's emitted triangles.
    ///
    /// `None` when the model emitted nothing.
    pub fn detect(triangles: &[[[f32; 3]; 3]]) -> Option<Self> {
        let mut z_min = f32::INFINITY;
        let mut z_max = f32::NEG_INFINITY;
        for tri in triangles {
            for v in tri {
                z_min = z_min.min(v[2]);
                z_max = z_max.max(v[2]);
            }
        }
        if !z_min.is_finite() || !z_max.is_finite() {
            return None;
        }
        Some(Self { z_min, z_max })
    }

    /// Whether `tri` is part of the hull's outer skin **and** buried
    /// under `ceiling`.
    ///
    /// `surf_normal` is the **authored** normal of the triangle's
    /// surface, in the same space as `tri`.
    pub fn is_buried_cap(
        &self,
        tri: &[[f32; 3]; 3],
        surf_normal: [f32; 3],
        ceiling: &TerrainCeiling,
    ) -> bool {
        self.is_outer_plane(tri, surf_normal) && ceiling.covers(tri)
    }

    /// Condition 1 alone — on the model's outer plane, facing outward.
    ///
    /// Public so a caller can measure how much the burial test is
    /// actually doing; **not** sufficient on its own to drop a face
    /// (see the module doc: on `Castle_CellBlock` it matches real
    /// floors).
    pub fn is_outer_plane(&self, tri: &[[f32; 3]; 3], surf_normal: [f32; 3]) -> bool {
        let len = (surf_normal[0] * surf_normal[0]
            + surf_normal[1] * surf_normal[1]
            + surf_normal[2] * surf_normal[2])
            .sqrt();
        if len < 1e-6 {
            return false;
        }
        let nz = surf_normal[2] / len;
        if nz.abs() < NEAR_HORIZONTAL_NZ {
            return false;
        }
        let plane = if nz > 0.0 { self.z_max } else { self.z_min };
        tri.iter()
            .all(|v| (v[2] - plane).abs() <= CAP_PLANE_EPSILON_CM)
    }
}

/// The lowest terrain height over each cell of a chunk's XY footprint.
///
/// Built from the terrain triangles the chunk already pushed into its
/// soup, so it costs one extra pass over vertices we have anyway.
///
/// "Lowest" rather than "highest" on purpose: [`Self::covers`] asks
/// *is the ground definitely above this face*, and taking the minimum
/// over a cell makes that test conservative — a cell straddling a
/// terrain hole edge answers with the lower rim.
#[derive(Debug, Clone)]
pub struct TerrainCeiling {
    origin: [f32; 2],
    cells_x: usize,
    cells_y: usize,
    /// `f32::INFINITY` where no terrain triangle touched the cell.
    min_z: Vec<f32>,
}

impl TerrainCeiling {
    /// Build a ceiling from world-space terrain triangles.
    ///
    /// `None` when there are none — the caller must then treat every
    /// face as unburied.
    pub fn from_triangles(
        triangles: impl IntoIterator<Item = [[f32; 3]; 3]> + Clone,
    ) -> Option<Self> {
        let (mut x0, mut y0) = (f32::INFINITY, f32::INFINITY);
        let (mut x1, mut y1) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        let mut any = false;
        for tri in triangles.clone() {
            for v in tri {
                x0 = x0.min(v[0]);
                y0 = y0.min(v[1]);
                x1 = x1.max(v[0]);
                y1 = y1.max(v[1]);
                any = true;
            }
        }
        if !any || !x0.is_finite() || !x1.is_finite() {
            return None;
        }
        let span = |lo: f32, hi: f32| {
            (((hi - lo) / CEILING_CELL_CM).ceil() as usize + 1).clamp(1, CEILING_MAX_CELLS)
        };
        let cells_x = span(x0, x1);
        let cells_y = span(y0, y1);
        let mut ceiling = Self {
            origin: [x0, y0],
            cells_x,
            cells_y,
            min_z: vec![f32::INFINITY; cells_x * cells_y],
        };
        // Conservative rasterisation: every cell the triangle's XY
        // bounding box touches records the triangle's lowest vertex.
        // Over-covering only ever lowers a cell's value, which can only
        // make `covers` answer "not buried".
        for tri in triangles {
            let tz = tri[0][2].min(tri[1][2]).min(tri[2][2]);
            let bx0 = tri[0][0].min(tri[1][0]).min(tri[2][0]);
            let bx1 = tri[0][0].max(tri[1][0]).max(tri[2][0]);
            let by0 = tri[0][1].min(tri[1][1]).min(tri[2][1]);
            let by1 = tri[0][1].max(tri[1][1]).max(tri[2][1]);
            for (ix, iy) in ceiling.cell_range(bx0, by0, bx1, by1) {
                let slot = &mut ceiling.min_z[iy * cells_x + ix];
                if tz < *slot {
                    *slot = tz;
                }
            }
        }
        Some(ceiling)
    }

    /// Whether terrain sits above every vertex of `tri`.
    ///
    /// `false` as soon as one vertex has no terrain over it — an
    /// unsampled cell is an opening, not a licence to delete.
    pub fn covers(&self, tri: &[[f32; 3]; 3]) -> bool {
        tri.iter().all(|v| match self.min_height_at(v[0], v[1]) {
            Some(z) => z > v[2],
            None => false,
        })
    }

    /// Lowest terrain height over the cell containing `(x, y)`.
    pub fn min_height_at(&self, x: f32, y: f32) -> Option<f32> {
        let (ix, iy) = self.cell_of(x, y)?;
        let z = self.min_z[iy * self.cells_x + ix];
        z.is_finite().then_some(z)
    }

    fn cell_of(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let fx = (x - self.origin[0]) / CEILING_CELL_CM;
        let fy = (y - self.origin[1]) / CEILING_CELL_CM;
        if fx < 0.0 || fy < 0.0 {
            return None;
        }
        let (ix, iy) = (fx as usize, fy as usize);
        (ix < self.cells_x && iy < self.cells_y).then_some((ix, iy))
    }

    fn cell_range(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(usize, usize)> {
        let lo = |v: f32, o: f32| (((v - o) / CEILING_CELL_CM).floor().max(0.0)) as usize;
        let (ix0, iy0) = (lo(x0, self.origin[0]), lo(y0, self.origin[1]));
        let ix1 = lo(x1, self.origin[0]).min(self.cells_x - 1);
        let iy1 = lo(y1, self.origin[1]).min(self.cells_y - 1);
        let mut out = Vec::new();
        for iy in iy0..=iy1.max(iy0) {
            for ix in ix0..=ix1.max(ix0) {
                if ix < self.cells_x && iy < self.cells_y {
                    out.push((ix, iy));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An axis-aligned horizontal triangle at height `z`.
    fn flat(z: f32) -> [[f32; 3]; 3] {
        [[0.0, 0.0, z], [1000.0, 0.0, z], [0.0, 1000.0, z]]
    }

    const UP: [f32; 3] = [0.0, 0.0, 1.0];
    const DOWN: [f32; 3] = [0.0, 0.0, -1.0];
    const SIDEWAYS: [f32; 3] = [1.0, 0.0, 0.0];

    /// The Castle interior hull: BW y 26.88 .. 79.36.
    fn castle_hull() -> HullCap {
        HullCap {
            z_min: 2_688.0,
            z_max: 7_936.0,
        }
    }

    /// Terrain sheet at height `z` covering the fixture's XY footprint.
    fn ground(z: f32) -> TerrainCeiling {
        TerrainCeiling::from_triangles(vec![
            [
                [-500.0, -500.0, z],
                [2000.0, -500.0, z],
                [-500.0, 2000.0, z],
            ],
            [
                [2000.0, 2000.0, z],
                [-500.0, 2000.0, z],
                [2000.0, -500.0, z],
            ],
        ])
        .expect("terrain fixture")
    }

    #[test]
    fn detect_reports_the_emitted_face_extent() {
        let cap = HullCap::detect(&[flat(2_688.0), flat(4_000.0), flat(7_936.0)]).expect("extent");
        assert_eq!(cap.z_min, 2_688.0);
        assert_eq!(cap.z_max, 7_936.0);
    }

    #[test]
    fn detect_ignores_the_empty_case() {
        assert_eq!(HullCap::detect(&[]), None);
    }

    #[test]
    fn the_top_plane_is_a_cap_only_facing_up() {
        let cap = castle_hull();
        assert!(
            cap.is_outer_plane(&flat(7_936.0), UP),
            "hull top faces up: skin"
        );
        assert!(
            !cap.is_outer_plane(&flat(7_936.0), DOWN),
            "a downward face on the top plane borders the hull interior — a ceiling, keep it"
        );
    }

    #[test]
    fn the_bottom_plane_is_a_cap_only_facing_down() {
        let cap = castle_hull();
        assert!(cap.is_outer_plane(&flat(2_688.0), DOWN));
        assert!(
            !cap.is_outer_plane(&flat(2_688.0), UP),
            "the lowest floor of the hull faces up and is walkable — keep it"
        );
    }

    #[test]
    fn interior_floors_are_never_caps() {
        // The throne room floor, BW y 38.08, buried under 130 m of rock.
        assert!(!castle_hull().is_buried_cap(&flat(3_808.0), UP, &ground(13_000.0)));
    }

    #[test]
    fn walls_are_never_caps() {
        // A vertical face whose vertices happen to touch the top plane.
        let wall = [
            [0.0, 0.0, 7_936.0],
            [1000.0, 0.0, 7_936.0],
            [0.0, 0.0, 7_936.0],
        ];
        assert!(!castle_hull().is_outer_plane(&wall, SIDEWAYS));
    }

    #[test]
    fn a_face_one_metre_below_the_plane_is_not_a_cap() {
        let cap = castle_hull();
        assert!(!cap.is_outer_plane(&flat(7_836.0), UP));
        // ...but authoring jitter inside the epsilon still counts.
        assert!(cap.is_outer_plane(&flat(7_935.5), UP));
    }

    #[test]
    fn a_degenerate_normal_is_not_a_cap() {
        assert!(!castle_hull().is_outer_plane(&flat(7_936.0), [0.0, 0.0, 0.0]));
    }

    // --- burial ------------------------------------------------------

    #[test]
    fn a_buried_hull_top_is_dropped() {
        // Castle: skin at BW y 79.36, terrain at 93.3.
        assert!(castle_hull().is_buried_cap(&flat(7_936.0), UP, &ground(9_330.0)));
    }

    #[test]
    fn an_outer_plane_above_the_terrain_is_kept() {
        // Castle_CellBlock: the 94.6 m sheet is the top of its model and
        // faces up, but the terrain there is a flat plane at BW y 0. It
        // is a real walkable floor in the shipped .nav and condition 1
        // alone would delete it.
        assert!(!castle_hull().is_buried_cap(&flat(7_936.0), UP, &ground(0.0)));
    }

    #[test]
    fn a_face_under_a_terrain_hole_is_kept() {
        // TID_Visibility_Off leaves no triangles, so the cell has no
        // sample. An opening is how a player reaches the surface — the
        // face stays.
        let hole_ring = TerrainCeiling::from_triangles(vec![[
            [-500.0, -500.0, 9_330.0],
            [-400.0, -500.0, 9_330.0],
            [-500.0, -400.0, 9_330.0],
        ]])
        .expect("terrain fixture");
        assert!(!castle_hull().is_buried_cap(&flat(7_936.0), UP, &hole_ring));
    }

    #[test]
    fn terrain_exactly_at_the_face_height_does_not_bury_it() {
        assert!(!castle_hull().is_buried_cap(&flat(7_936.0), UP, &ground(7_936.0)));
    }

    #[test]
    fn a_chunk_with_no_terrain_has_no_ceiling() {
        assert!(TerrainCeiling::from_triangles(Vec::<[[f32; 3]; 3]>::new()).is_none());
    }

    #[test]
    fn the_ceiling_records_the_lowest_terrain_in_a_cell() {
        // Two overlapping terrain triangles in the same cell: the
        // conservative answer is the lower one.
        let c = TerrainCeiling::from_triangles(vec![
            [
                [0.0, 0.0, 5_000.0],
                [100.0, 0.0, 5_000.0],
                [0.0, 100.0, 5_000.0],
            ],
            [
                [0.0, 0.0, 3_000.0],
                [100.0, 0.0, 3_000.0],
                [0.0, 100.0, 3_000.0],
            ],
        ])
        .expect("terrain fixture");
        assert_eq!(c.min_height_at(50.0, 50.0), Some(3_000.0));
    }

    #[test]
    fn the_ceiling_reports_nothing_outside_its_footprint() {
        let c = ground(9_330.0);
        assert!(c.min_height_at(-10_000.0, 0.0).is_none());
        assert!(c.min_height_at(0.0, 99_999.0).is_none());
    }
}

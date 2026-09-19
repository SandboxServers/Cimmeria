//! Public data types for a decoded UE3 `UModel` / `UPolys` plus the
//! node-fan triangulator the navmesh extractor consumes.
//!
//! The on-disk layout these structs are populated from is documented on
//! the [`crate::model`] module and in [`super::parse`].

/// `FBoxSphereBounds` — 28 raw bytes at the head of a `Model`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModelBounds {
    pub origin: [f32; 3],
    pub box_extent: [f32; 3],
    pub sphere_radius: f32,
}

/// One `FBspNode` (68 bytes on the wire).
///
/// Only the fields needed to reconstruct collision triangles are
/// decoded. Offsets `+0x18` (`iVertPool`), `+0x1c` (`iSurf`), `+0x3a`
/// (`NumVertices`) and `+0x3b` (`NodeFlags`) are HIGH confidence — the
/// first two from a 399/399 bounds-check sweep on real data, the last
/// two directly from the decompiled `UModel::PointClassify` BSP walker.
/// `Plane` at `+0x00` is MEDIUM (canonical UE3 order, consistent with
/// the plane-read helper call) and is carried for debugging only.
#[derive(Debug, Clone, Copy, Default)]
pub struct BspNode {
    /// Splitting plane (X, Y, Z, W). Not used by the triangulator.
    pub plane: [f32; 4],
    /// First index into `Model::verts` for this node's face.
    pub i_vert_pool: i32,
    /// Index into `Model::surfs` for this node's surface properties.
    pub i_surf: i32,
    /// Number of `FVert` entries forming this node's convex face.
    /// Zero means "pure BSP splitter, carries no geometry".
    pub num_vertices: u8,
    /// `EBspNodeFlags` bitfield. The loader masks this with `0x1f`, so
    /// only bits 0..4 are ever observable post-load.
    pub node_flags: u8,
}

/// One `FBspSurf` (56 bytes on the wire when `ArVer > 0x1a0`).
///
/// Field offsets are HIGH confidence — `FBspSurf_Serialize` was
/// decompiled field-by-field.
#[derive(Debug, Clone, Copy, Default)]
pub struct BspSurf {
    /// `UMaterialInterface*` object reference.
    pub material_ref: i32,
    /// `EPolyFlags` bitfield — the solidity/visibility filter input.
    pub poly_flags: u32,
    /// Vertex-pool index of the plane's base point.
    pub p_base: i32,
    /// Index into `Model::vectors` for the surface normal.
    pub v_normal: i32,
    /// Index into the owning brush's `Polys->Element`.
    pub i_brush_poly: i32,
    /// Owning `ABrush*` object reference.
    pub actor_ref: i32,
    /// Surface plane (X, Y, Z, W).
    pub plane: [f32; 4],
}

/// One `FVert` (24 bytes on the wire).
///
/// Only `pVertex` (the first 4 bytes — an index into `Model::points`)
/// matters for geometry recovery. The remaining 20 bytes
/// (`iSide` + two shadow-map texture coordinate pairs, by UE3
/// convention and stride fit) are skipped.
#[derive(Debug, Clone, Copy, Default)]
pub struct BspVert {
    pub p_vertex: i32,
}

// --- EPolyFlags (`FBspSurf::PolyFlags`) --------------------------------
//
// ASSUMED from the public UE3 SDK — the exact bit-to-flag mapping in
// this SGW build was NOT independently re-derived from the binary (see
// the "Open questions" table in
// `docs/reverse-engineering/findings/bsp-model-polys-serialize.md`).
// Because the assumption is unverified, the triangulator reports a full
// per-bit exclusion breakdown alongside its output so a wrong mapping
// shows up as an implausible drop count instead of silently punching a
// hole in the navmesh.

/// Poly is invisible.
pub const PF_INVISIBLE: u32 = 0x0000_0001;
/// Poly does not block movement.
pub const PF_NOT_SOLID: u32 = 0x0000_0008;
/// Collision-solid but CSG-nonsolid. **Not** excluded — it still blocks.
pub const PF_SEMISOLID: u32 = 0x0000_0020;
/// Visible from both sides. **Not** excluded — it still blocks.
pub const PF_TWO_SIDED: u32 = 0x0000_0100;
/// Portal between zones — a visibility construct, not geometry.
pub const PF_PORTAL: u32 = 0x0400_0000;

// --- EBspNodeFlags (`FBspNode::NodeFlags`) -----------------------------

/// Node is not part of the CSG solid set.
pub const NF_NOT_CSG: u8 = 0x01;
/// Projectiles pass through.
pub const NF_SHOOT_THROUGH: u8 = 0x02;
/// Does not block visibility.
pub const NF_NOT_VIS_BLOCKING: u8 = 0x04;

/// The `PolyFlags` bits treated as "this surface does not block".
///
/// **Change this table, not the triangulator**, when the PolyFlags
/// semantics are pinned down for real. Every entry is reported
/// individually by [`BspTriangulation::excluded_by_flag`], so adding a
/// speculative bit here and reading the count back off a real tile is
/// the intended way to test a hypothesis.
pub const NON_COLLIDING_POLY_FLAGS: &[(&str, u32)] = &[
    ("PF_Invisible", PF_INVISIBLE),
    ("PF_NotSolid", PF_NOT_SOLID),
    ("PF_Portal", PF_PORTAL),
];

/// The `NodeFlags` bits treated as "this node does not block".
///
/// Deliberately **empty by default**. `NF_NotCsg` is the obvious
/// candidate (the decompiled collision walker tests `flags & 0x21`, and
/// the loader masks bit 5 off, leaving bit 0), but "not part of the CSG
/// solid set" is not the same claim as "not collidable" — a brush-local
/// `Model` has every node outside the level's CSG set. Excluding it
/// blind would delete all the per-`Brush` geometry. The counts are
/// reported so the hypothesis stays visible.
pub const NON_COLLIDING_NODE_FLAGS: &[(&str, u8)] = &[];

/// Which surfaces the triangulator drops.
///
/// Defaults to the [`NON_COLLIDING_POLY_FLAGS`] /
/// [`NON_COLLIDING_NODE_FLAGS`] tables above; construct one by hand to
/// test a different hypothesis without touching the tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionFilter {
    /// A node is dropped when `Surfs[iSurf].PolyFlags & poly_flag_mask != 0`.
    pub poly_flag_mask: u32,
    /// A node is dropped when `node.NodeFlags & node_flag_mask != 0`.
    pub node_flag_mask: u8,
}

impl CollisionFilter {
    /// A filter that keeps everything — the baseline used to report the
    /// unfiltered triangle count.
    pub const KEEP_ALL: Self = Self {
        poly_flag_mask: 0,
        node_flag_mask: 0,
    };
}

impl Default for CollisionFilter {
    fn default() -> Self {
        let mut poly_flag_mask = 0u32;
        let mut i = 0;
        // `const`-friendly fold: NON_COLLIDING_POLY_FLAGS is a slice, so
        // iterator adaptors aren't available in a const context and a
        // plain loop keeps this readable either way.
        while i < NON_COLLIDING_POLY_FLAGS.len() {
            poly_flag_mask |= NON_COLLIDING_POLY_FLAGS[i].1;
            i += 1;
        }
        let mut node_flag_mask = 0u8;
        let mut j = 0;
        while j < NON_COLLIDING_NODE_FLAGS.len() {
            node_flag_mask |= NON_COLLIDING_NODE_FLAGS[j].1;
            j += 1;
        }
        Self {
            poly_flag_mask,
            node_flag_mask,
        }
    }
}

/// A decoded UE3 `UModel`.
#[derive(Debug, Default)]
pub struct Model {
    pub bounds: ModelBounds,
    /// Direction/normal pool — indexed by `BspSurf::v_normal`.
    pub vectors: Vec<[f32; 3]>,
    /// Position pool — indexed by `BspVert::p_vertex`.
    pub points: Vec<[f32; 3]>,
    pub nodes: Vec<BspNode>,
    pub surfs: Vec<BspSurf>,
    pub verts: Vec<BspVert>,
    pub num_shared_sides: i32,
    pub num_zones: i32,
    /// Object reference to this model's `UPolys` export. Always
    /// populated in cooked SGW packages — `Polys` is **not** stripped.
    pub polys_ref: i32,
    pub root_outside: bool,
    pub linked: bool,
}

/// Result of fan-triangulating a `Model`'s BSP nodes, with enough
/// bookkeeping to make a wrong flag assumption visible.
#[derive(Debug, Default)]
pub struct BspTriangulation {
    /// Model-local triangles that survived the filter.
    pub triangles: Vec<[[f32; 3]; 3]>,
    /// Parallel to [`Self::triangles`]: the `Surfs` index each triangle
    /// came from. Lets a caller recover the *true* surface normal
    /// (via [`Model::surf_normal`]) independently of emitted winding —
    /// which is what the floor probe needs, since "is there a floor
    /// here" must not depend on the fan direction.
    pub triangle_surf: Vec<u32>,
    /// Total `Nodes` entries walked.
    pub nodes_total: usize,
    /// Nodes with `NumVertices == 0` (pure BSP splitters, no face).
    pub nodes_without_vertices: usize,
    /// Nodes dropped because their surface matched the filter.
    pub nodes_excluded: usize,
    /// Nodes dropped because `iSurf` / `iVertPool` / `pVertex` pointed
    /// outside their arrays. Non-zero means the parse is wrong.
    pub nodes_out_of_range: usize,
    /// Nodes dropped because `NumVertices < 3` (a face needs at least a
    /// triangle; 1- and 2-vertex nodes fan to nothing).
    pub nodes_degenerate: usize,
    /// Triangles the filter removed (i.e. how many more you'd get with
    /// [`CollisionFilter::KEEP_ALL`]).
    pub triangles_excluded: usize,
    /// Per-named-flag triangle exclusion counts. Each entry counts the
    /// triangles that bit *alone* would remove, so overlapping bits
    /// double-count — that's intentional, it keeps each hypothesis
    /// independently readable.
    pub excluded_by_flag: Vec<(&'static str, u32, usize)>,
    /// `(PolyFlags value, face-carrying node count)`, descending by
    /// count. Use this to spot flag values the table doesn't explain.
    pub poly_flag_histogram: Vec<(u32, usize)>,
    /// `(NodeFlags value, face-carrying node count)`, descending.
    pub node_flag_histogram: Vec<(u8, usize)>,
}

impl Model {
    /// Fan-triangulate every face-carrying BSP node into model-local
    /// triangles.
    ///
    /// For each `Nodes[i]` with `NumVertices >= 3`, the face's corner
    /// positions are `Points[Verts[iVertPool + k].pVertex]` for
    /// `k in 0..NumVertices`; BSP node polygons are convex by
    /// construction, so a `(0, k, k+1)` fan is a valid triangulation.
    ///
    /// Nodes whose indices fall outside their arrays are dropped and
    /// counted rather than panicking — but a non-zero
    /// [`BspTriangulation::nodes_out_of_range`] on real data means the
    /// deserializer's field offsets are wrong, not that the data is
    /// malformed.
    pub fn triangulate(&self, filter: CollisionFilter) -> BspTriangulation {
        let mut out = BspTriangulation {
            nodes_total: self.nodes.len(),
            ..Default::default()
        };

        let mut poly_hist: Vec<(u32, usize)> = Vec::new();
        let mut node_hist: Vec<(u8, usize)> = Vec::new();
        let mut per_flag: Vec<(&'static str, u32, usize)> = NON_COLLIDING_POLY_FLAGS
            .iter()
            .map(|(n, b)| (*n, *b, 0usize))
            .collect();
        let mut per_node_flag: Vec<(&'static str, u32, usize)> = NON_COLLIDING_NODE_FLAGS
            .iter()
            .map(|(n, b)| (*n, *b as u32, 0usize))
            .collect();

        for node in &self.nodes {
            if node.num_vertices == 0 {
                out.nodes_without_vertices += 1;
                continue;
            }
            let Some(surf_index) = usize::try_from(node.i_surf).ok() else {
                out.nodes_out_of_range += 1;
                continue;
            };
            let Some(surf) = self.surfs.get(surf_index) else {
                out.nodes_out_of_range += 1;
                continue;
            };

            bump(&mut poly_hist, surf.poly_flags);
            bump(&mut node_hist, node.node_flags);

            // Gather the face's corner positions first: the per-flag
            // exclusion counts are in *triangles*, so we need to know
            // how many triangles this node would have produced before
            // deciding it was filtered out.
            let Some(corners) = self.face_corners(node) else {
                out.nodes_out_of_range += 1;
                continue;
            };
            if corners.len() < 3 {
                out.nodes_degenerate += 1;
                continue;
            }
            let tri_count = corners.len() - 2;

            for entry in per_flag.iter_mut() {
                if surf.poly_flags & entry.1 != 0 {
                    entry.2 += tri_count;
                }
            }
            for entry in per_node_flag.iter_mut() {
                if (node.node_flags as u32) & entry.1 != 0 {
                    entry.2 += tri_count;
                }
            }

            if surf.poly_flags & filter.poly_flag_mask != 0
                || node.node_flags & filter.node_flag_mask != 0
            {
                out.nodes_excluded += 1;
                out.triangles_excluded += tri_count;
                continue;
            }

            for k in 1..corners.len() - 1 {
                out.triangles.push([corners[0], corners[k], corners[k + 1]]);
                out.triangle_surf.push(surf_index as u32);
            }
        }

        per_flag.append(&mut per_node_flag);
        out.excluded_by_flag = per_flag;
        poly_hist.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        node_hist.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        out.poly_flag_histogram = poly_hist;
        out.node_flag_histogram = node_hist;
        out
    }

    /// The authored (not winding-derived) outward normal of a surface,
    /// in model-local space.
    ///
    /// Prefers `Vectors[vNormal]` — the pooled normal the engine
    /// authored — and falls back to the XYZ of `FBspSurf::Plane` when
    /// the pool index is out of range. Callers that need to know
    /// "which way does this face point" must use this rather than a
    /// cross product of the emitted triangle, because the emitted fan
    /// winding is a property of the node vertex pool, not of the
    /// surface.
    pub fn surf_normal(&self, surf_index: usize) -> Option<[f32; 3]> {
        let surf = self.surfs.get(surf_index)?;
        if let Some(n) = usize::try_from(surf.v_normal)
            .ok()
            .and_then(|i| self.vectors.get(i))
        {
            return Some(*n);
        }
        Some([surf.plane[0], surf.plane[1], surf.plane[2]])
    }

    /// Resolve one node's face into model-local corner positions.
    ///
    /// Returns `None` if any index (`iVertPool`, `pVertex`) falls
    /// outside its array — the caller counts that as an out-of-range
    /// node rather than emitting a partial face.
    fn face_corners(&self, node: &BspNode) -> Option<Vec<[f32; 3]>> {
        let start = usize::try_from(node.i_vert_pool).ok()?;
        let n = node.num_vertices as usize;
        let slice = self.verts.get(start..start.checked_add(n)?)?;
        let mut corners = Vec::with_capacity(n);
        for v in slice {
            let p = usize::try_from(v.p_vertex).ok()?;
            corners.push(*self.points.get(p)?);
        }
        Some(corners)
    }
}

/// Increment the count for `value` in a `(value, count)` histogram.
fn bump<T: PartialEq + Copy>(hist: &mut Vec<(T, usize)>, value: T) {
    if let Some(e) = hist.iter_mut().find(|e| e.0 == value) {
        e.1 += 1;
    } else {
        hist.push((value, 1));
    }
}

/// One `FPoly` — an original, un-split CSG brush polygon.
#[derive(Debug, Clone, Default)]
pub struct Poly {
    pub base: [f32; 3],
    pub normal: [f32; 3],
    pub texture_u: [f32; 3],
    pub texture_v: [f32; 3],
    pub vertices: Vec<[f32; 3]>,
    pub poly_flags: u32,
    /// Owning `ABrush*` object reference.
    pub actor_ref: i32,
    /// `UMaterialInterface*` object reference.
    pub material_ref: i32,
}

impl Poly {
    /// Fan-triangulate this (convex) polygon into model-local triangles.
    pub fn triangles(&self) -> Vec<[[f32; 3]; 3]> {
        if self.vertices.len() < 3 {
            return Vec::new();
        }
        (1..self.vertices.len() - 1)
            .map(|k| [self.vertices[0], self.vertices[k], self.vertices[k + 1]])
            .collect()
    }
}

/// A decoded UE3 `UPolys` — the `Element` array of [`Poly`] records.
#[derive(Debug, Default)]
pub struct Polys {
    pub elements: Vec<Poly>,
}

impl Polys {
    /// Total triangle count across every element, unfiltered. Used as a
    /// cross-check against the `Nodes`-derived count.
    pub fn triangle_count(&self) -> usize {
        self.elements
            .iter()
            .map(|p| p.vertices.len().saturating_sub(2))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model with one square face in the XY plane at z = 0, wound
    /// counter-clockwise when viewed from +Z. Right-hand-rule normal
    /// of that order is +Z.
    fn square_model(poly_flags: u32, node_flags: u8) -> Model {
        Model {
            vectors: vec![[0.0, 0.0, 1.0]],
            points: vec![
                [0.0, 0.0, 0.0],
                [100.0, 0.0, 0.0],
                [100.0, 100.0, 0.0],
                [0.0, 100.0, 0.0],
            ],
            verts: vec![
                BspVert { p_vertex: 0 },
                BspVert { p_vertex: 1 },
                BspVert { p_vertex: 2 },
                BspVert { p_vertex: 3 },
            ],
            surfs: vec![BspSurf {
                poly_flags,
                v_normal: 0,
                plane: [0.0, 0.0, 1.0, 0.0],
                ..Default::default()
            }],
            nodes: vec![BspNode {
                i_vert_pool: 0,
                i_surf: 0,
                num_vertices: 4,
                node_flags,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn normal_of(t: [[f32; 3]; 3]) -> [f32; 3] {
        let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
        let v = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }

    #[test]
    fn quad_node_fans_into_two_triangles_in_vert_pool_order() {
        // WINDING PIN. The fan is `(corners[0], corners[k], corners[k+1])`
        // over the node's `iVertPool` slice **as stored** — the
        // triangulator never reverses. `bsp.rs`'s real-data winding
        // check (floors must emit with UE3 n.z < 0 so NavBuilder sees
        // them as walkable) is only meaningful if this stays true, so a
        // change here must be a deliberate, re-measured decision.
        let m = square_model(0, 0);
        let t = m.triangulate(CollisionFilter::default());
        assert_eq!(t.triangles.len(), 2);
        assert_eq!(
            t.triangles[0],
            [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [100.0, 100.0, 0.0]]
        );
        assert_eq!(
            t.triangles[1],
            [[0.0, 0.0, 0.0], [100.0, 100.0, 0.0], [0.0, 100.0, 0.0]]
        );
        // Both emitted triangles carry the same right-hand-rule normal
        // direction as the stored vertex order (+Z here).
        assert!(normal_of(t.triangles[0])[2] > 0.0);
        assert!(normal_of(t.triangles[1])[2] > 0.0);
        assert_eq!(t.triangle_surf, vec![0, 0]);
    }

    #[test]
    fn surf_normal_is_independent_of_emitted_winding() {
        // The floor probe must not depend on fan direction: it reads
        // the authored normal out of `Vectors[vNormal]`.
        let mut m = square_model(0, 0);
        m.vectors = vec![[0.0, 0.0, -1.0]];
        let t = m.triangulate(CollisionFilter::default());
        // Emitted winding still says +Z…
        assert!(normal_of(t.triangles[0])[2] > 0.0);
        // …but the authored surface normal says -Z.
        assert_eq!(m.surf_normal(0), Some([0.0, 0.0, -1.0]));
    }

    #[test]
    fn surf_normal_falls_back_to_the_plane_when_vnormal_is_out_of_range() {
        let mut m = square_model(0, 0);
        m.surfs[0].v_normal = 99;
        assert_eq!(m.surf_normal(0), Some([0.0, 0.0, 1.0]));
    }

    #[test]
    fn default_filter_drops_invisible_and_not_solid_surfaces() {
        for bit in [PF_INVISIBLE, PF_NOT_SOLID, PF_PORTAL] {
            let m = square_model(bit, 0);
            let t = m.triangulate(CollisionFilter::default());
            assert!(
                t.triangles.is_empty(),
                "flag {bit:#x} should have been filtered out"
            );
            assert_eq!(t.nodes_excluded, 1);
            assert_eq!(t.triangles_excluded, 2);
        }
    }

    #[test]
    fn default_filter_keeps_semisolid_and_two_sided_surfaces() {
        // Both still block; excluding them would punch holes in walls.
        for bit in [PF_SEMISOLID, PF_TWO_SIDED, 0x0000_0E00, 0x0000_0200] {
            let m = square_model(bit, 0);
            let t = m.triangulate(CollisionFilter::default());
            assert_eq!(t.triangles.len(), 2, "flag {bit:#x} should be kept");
        }
    }

    #[test]
    fn keep_all_filter_emits_everything() {
        let m = square_model(PF_INVISIBLE | PF_NOT_SOLID, 0xff);
        let t = m.triangulate(CollisionFilter::KEEP_ALL);
        assert_eq!(t.triangles.len(), 2);
        assert_eq!(t.triangles_excluded, 0);
    }

    #[test]
    fn per_flag_exclusion_counts_are_reported_even_when_the_flag_is_not_filtered() {
        // This is the "make a wrong assumption visible" mechanism: the
        // counts are computed for every entry in the table regardless
        // of whether the active filter uses that bit.
        let m = square_model(PF_INVISIBLE, 0);
        let t = m.triangulate(CollisionFilter::KEEP_ALL);
        let inv = t
            .excluded_by_flag
            .iter()
            .find(|e| e.0 == "PF_Invisible")
            .expect("PF_Invisible must be in the report table");
        assert_eq!(inv.2, 2, "both fan triangles carry the flag");
        let ns = t
            .excluded_by_flag
            .iter()
            .find(|e| e.0 == "PF_NotSolid")
            .expect("PF_NotSolid must be in the report table");
        assert_eq!(ns.2, 0);
    }

    #[test]
    fn node_flag_filter_is_off_by_default_but_configurable() {
        let m = square_model(0, NF_NOT_CSG);
        assert_eq!(
            m.triangulate(CollisionFilter::default()).triangles.len(),
            2,
            "NF_NotCsg must NOT be excluded by default — every brush-local \
             Model node is outside the level's CSG set"
        );
        let strict = CollisionFilter {
            poly_flag_mask: 0,
            node_flag_mask: NF_NOT_CSG,
        };
        assert!(m.triangulate(strict).triangles.is_empty());
    }

    #[test]
    fn splitter_nodes_with_zero_vertices_are_counted_not_emitted() {
        let mut m = square_model(0, 0);
        m.nodes[0].num_vertices = 0;
        let t = m.triangulate(CollisionFilter::default());
        assert!(t.triangles.is_empty());
        assert_eq!(t.nodes_without_vertices, 1);
        assert_eq!(t.nodes_out_of_range, 0);
    }

    #[test]
    fn nodes_with_fewer_than_three_vertices_are_degenerate() {
        let mut m = square_model(0, 0);
        m.nodes[0].num_vertices = 2;
        let t = m.triangulate(CollisionFilter::default());
        assert!(t.triangles.is_empty());
        assert_eq!(t.nodes_degenerate, 1);
    }

    #[test]
    fn out_of_range_indices_are_counted_not_panicked_on() {
        // A bad iSurf, a bad iVertPool and a bad pVertex each drop the
        // node. Non-zero `nodes_out_of_range` on real data means the
        // deserializer's field offsets are wrong.
        let mut bad_surf = square_model(0, 0);
        bad_surf.nodes[0].i_surf = 99;
        assert_eq!(
            bad_surf
                .triangulate(CollisionFilter::default())
                .nodes_out_of_range,
            1
        );

        let mut bad_pool = square_model(0, 0);
        bad_pool.nodes[0].i_vert_pool = 3; // 3 + 4 > verts.len()
        assert_eq!(
            bad_pool
                .triangulate(CollisionFilter::default())
                .nodes_out_of_range,
            1
        );

        let mut bad_point = square_model(0, 0);
        bad_point.verts[2].p_vertex = 77;
        assert_eq!(
            bad_point
                .triangulate(CollisionFilter::default())
                .nodes_out_of_range,
            1
        );

        let mut negative = square_model(0, 0);
        negative.nodes[0].i_vert_pool = -1;
        assert_eq!(
            negative
                .triangulate(CollisionFilter::default())
                .nodes_out_of_range,
            1
        );
    }

    #[test]
    fn histograms_group_face_carrying_nodes_by_flag_value() {
        let mut m = square_model(0x0000_0E00, 0);
        // Second surf/node pair with a different PolyFlags value.
        m.surfs.push(BspSurf {
            poly_flags: 0x0000_0200,
            v_normal: 0,
            ..Default::default()
        });
        m.nodes.push(BspNode {
            i_vert_pool: 0,
            i_surf: 1,
            num_vertices: 4,
            node_flags: 0,
            ..Default::default()
        });
        m.nodes.push(BspNode {
            i_vert_pool: 0,
            i_surf: 1,
            num_vertices: 4,
            node_flags: 0,
            ..Default::default()
        });
        let t = m.triangulate(CollisionFilter::default());
        // Descending by count: 0x200 appears on two nodes, 0xE00 on one.
        assert_eq!(
            t.poly_flag_histogram,
            vec![(0x0000_0200, 2), (0x0000_0E00, 1)]
        );
        assert_eq!(t.node_flag_histogram, vec![(0u8, 3)]);
        assert_eq!(t.nodes_total, 3);
        assert_eq!(t.triangles.len(), 6);
    }

    #[test]
    fn default_filter_mask_matches_the_published_table() {
        // Guards the const-fold in `Default` against the table drifting
        // out from under it.
        let expected = NON_COLLIDING_POLY_FLAGS.iter().fold(0u32, |a, e| a | e.1);
        assert_eq!(CollisionFilter::default().poly_flag_mask, expected);
        assert_eq!(CollisionFilter::default().node_flag_mask, 0);
    }

    #[test]
    fn poly_fan_matches_node_fan_for_the_same_corner_order() {
        // The Polys path is a redundant copy of the same surfaces; a
        // quad must fan the same way from either source.
        let p = Poly {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [100.0, 0.0, 0.0],
                [100.0, 100.0, 0.0],
                [0.0, 100.0, 0.0],
            ],
            ..Default::default()
        };
        let node_tris = square_model(0, 0)
            .triangulate(CollisionFilter::default())
            .triangles;
        assert_eq!(p.triangles(), node_tris);
    }

    #[test]
    fn degenerate_poly_fans_to_nothing() {
        let p = Poly {
            vertices: vec![[0.0; 3], [1.0, 0.0, 0.0]],
            ..Default::default()
        };
        assert!(p.triangles().is_empty());
        assert_eq!(Polys { elements: vec![p] }.triangle_count(), 0);
    }
}

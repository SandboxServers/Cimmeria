//! In-memory triangle soup + per-chunk world-space offset arithmetic.
//!
//! The extractor accumulates triangles into a [`TriangleSoup`] per
//! chunk, then [`crate::obj`] flushes that buffer to a `.obj` file
//! using NavBuilder's expected vertex convention. Per-chunk OBJ files
//! are emitted in **UE3 cm coordinates** — NavBuilder's
//! `Mesh::loadOBJ` re-swizzles them into BW post-divide units on read
//! (`v.x = obj.z/100, v.y = obj.y/100, v.z = obj.x/100`).
//!
//! Phase 0 ships only the data structures + transform plumbing.
//! Phase 1.2 fills `static_mesh_instance_triangles`. Phase 1.3 fills
//! `triangulate_terrain`.

/// Three vertices forming a single triangle, in UE3 cm coordinates.
pub type Triangle = [[f32; 3]; 3];

/// Accumulator for triangles destined for a single chunk's OBJ file.
///
/// Vertices are stored once and indexed by 1-based OBJ-style indices
/// inside `faces` so the emitter doesn't have to dedupe on the fly.
/// The naïve "no dedupe" strategy is fine here — chunks emit at most
/// a few hundred thousand triangles, and NavBuilder rebuilds its own
/// adjacency from the OBJ regardless.
#[derive(Debug, Default, Clone)]
pub struct TriangleSoup {
    /// All vertices, in declaration order.
    pub vertices: Vec<[f32; 3]>,
    /// One face per triangle — three 1-based indices into `vertices`,
    /// matching the OBJ format.
    pub faces: Vec<[u32; 3]>,
    /// Optional human-readable group label (becomes an `o` line in the
    /// OBJ). Used to split per-actor groups so NavBuilder can filter
    /// `Terrain_*` groups via its existing `ignore` heuristic in
    /// `mesh.cpp:88-96`.
    pub group: Option<String>,
}

impl TriangleSoup {
    /// Build an empty soup with an optional group name.
    pub fn new(group: Option<String>) -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
            group,
        }
    }

    /// Push a single triangle. Vertices are appended; the face uses
    /// the next three 1-based indices.
    pub fn push(&mut self, tri: Triangle) {
        let base = self.vertices.len() as u32;
        self.vertices.push(tri[0]);
        self.vertices.push(tri[1]);
        self.vertices.push(tri[2]);
        // OBJ uses 1-based vertex indices.
        self.faces.push([base + 1, base + 2, base + 3]);
    }

    /// Number of triangles currently in the soup.
    pub fn triangle_count(&self) -> usize {
        self.faces.len()
    }
}

/// Apply a translation to every triangle (vertex-by-vertex).
///
/// Used by the per-chunk extractor to move chunk-local actor vertices
/// into world space when an export's `Location` is stored relative
/// to the chunk origin. (For the Castle_CellBlock dataset every
/// inspected Terrain export has `Location = (0,0,0)`, so the chunk
/// filename's grid offset is the only translation in play — but we
/// keep the per-triangle translate routine general for non-cellblock
/// maps that DO emit non-zero locations.)
pub fn translate_triangles(soup: &mut TriangleSoup, offset: [f32; 3]) {
    for v in &mut soup.vertices {
        v[0] += offset[0];
        v[1] += offset[1];
        v[2] += offset[2];
    }
}

/// Triangulate a UE3 terrain patch into world-space triangles.
///
/// **Stub for Phase 1.3.** The recipe is documented in
/// `.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`:
///
/// - Two triangles per cell `(i, j)`: `(v00, v10, v11)` and `(v00, v11, v01)`.
/// - `idx(i, j) = j * NumVerticesX + i`.
/// - `z = location.z + (height_u16 / 65535.0) * draw_scale * draw_scale_3d.z * 256.0`.
/// - Skip any cell where any of its four vertices has `InfoData[idx] & 0x01 != 0`
///   (`TERRAINFLAG_Invisible`).
///
/// Returns an empty soup until Phase 1.3 lands. The signature is set
/// up now so the orchestrator's call site doesn't need to change when
/// the body is filled in.
#[allow(clippy::too_many_arguments)]
pub fn triangulate_terrain(
    _heights: &[u16],
    _info_data: &[u8],
    _num_vertices_x: i32,
    _num_vertices_y: i32,
    _location: [f32; 3],
    _draw_scale: f32,
    _draw_scale_3d: [f32; 3],
) -> TriangleSoup {
    TriangleSoup::new(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_appends_three_vertices_one_face() {
        let mut soup = TriangleSoup::new(None);
        soup.push([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        assert_eq!(soup.vertices.len(), 3);
        assert_eq!(soup.faces, vec![[1, 2, 3]]);
        assert_eq!(soup.triangle_count(), 1);
    }

    #[test]
    fn push_two_triangles_indices_advance() {
        let mut soup = TriangleSoup::new(None);
        soup.push([[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        soup.push([[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        assert_eq!(soup.faces, vec![[1, 2, 3], [4, 5, 6]]);
    }

    #[test]
    fn translate_offsets_every_vertex() {
        let mut soup = TriangleSoup::new(None);
        soup.push([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        translate_triangles(&mut soup, [10.0, 20.0, 30.0]);
        assert_eq!(soup.vertices[0], [10.0, 20.0, 30.0]);
        assert_eq!(soup.vertices[1], [11.0, 20.0, 30.0]);
        assert_eq!(soup.vertices[2], [10.0, 21.0, 30.0]);
    }

    #[test]
    fn triangulate_terrain_stub_returns_empty() {
        // Phase 1.3 follow-up will replace this assertion. Until then
        // the stub must return an empty soup so the orchestrator's
        // no-op loop terminates cleanly.
        let soup = triangulate_terrain(&[], &[], 21, 21, [0.0; 3], 1.0, [1.0; 3]);
        assert_eq!(soup.triangle_count(), 0);
    }
}

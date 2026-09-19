//! In-memory triangle soup + per-chunk world-space offset arithmetic.
//!
//! The extractor accumulates triangles into a [`TriangleSoup`] per
//! chunk, then [`crate::obj`] flushes that buffer to a `.obj` file
//! using NavBuilder's expected vertex convention. Per-chunk OBJ files
//! are emitted in **UE3 cm coordinates** — NavBuilder's
//! `Mesh::loadOBJ` re-swizzles them into BW post-divide units on read
//! (`v.x = obj.z/100, v.y = obj.y/100, v.z = obj.x/100`).
//!
//! This module owns only the accumulator and the translate helper.
//! Triangle *producers* live next to the decoder they consume:
//! [`crate::staticmesh`] for `StaticMeshActor`, [`crate::terrain`] for
//! `Terrain`.

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
/// Kept for callers that need to move chunk-local vertices into world
/// space. Neither of the two extractors needs it today: both
/// `StaticMeshActor` and `Terrain` exports store an **absolute
/// world-space** `Location`. (An earlier revision of this comment
/// claimed every Castle_CellBlock `Terrain` sits at `(0,0,0)`; that is
/// wrong — the 25 terrains in chunk `00000000` are at
/// `(0..8000, 0..8000, 0)` and the map's 1600 terrains tile an exact
/// 2000 cm grid from -40000 to +38000 on both axes.)
pub fn translate_triangles(soup: &mut TriangleSoup, offset: [f32; 3]) {
    for v in &mut soup.vertices {
        v[0] += offset[0];
        v[1] += offset[1];
        v[2] += offset[2];
    }
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
}

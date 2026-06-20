//! Public data types for a decoded UE3 StaticMesh and the collision-triangle
//! accessor used by the navmesh extractor.
//!
//! The on-disk vertex/buffer layout these structs are populated from is
//! documented on the [`crate::static_mesh`] module and in [`super::parse`].

/// A decoded UE3 StaticMesh object.
#[derive(Debug)]
pub struct StaticMesh {
    pub bounds: BoundingBox,
    pub lod_models: Vec<LodModel>,
    pub internal_version: i32,
    /// kDOP collision triangles parsed from the kDOPTree.
    ///
    /// Each entry is `(v0, v1, v2, material)` where the three vertex indices
    /// reference the LOD0 vertex buffer. SGW cooked StaticMeshes typically
    /// populate this with a collision-flagged subset of the render triangles;
    /// when the array is empty (e.g. a non-colliding mesh), callers should
    /// fall back to the LOD0 index buffer to recover a triangle list.
    ///
    /// Phase 1.2 of the navmesh extractor consumes these via
    /// [`StaticMesh::collision_triangles`].
    pub kdop_triangles: Vec<KdopTriangle>,
}

/// One collision triangle from the kDOPTree.
///
/// Vertex indices reference the LOD0 vertex buffer's `positions[]`. The
/// material index is a per-section identifier we do not currently use —
/// kept on the struct for parity with the on-disk record.
#[derive(Debug, Clone, Copy)]
pub struct KdopTriangle {
    pub v0: u16,
    pub v1: u16,
    pub v2: u16,
    pub material: u16,
}

/// Axis-aligned bounding box with sphere radius.
#[derive(Debug)]
pub struct BoundingBox {
    pub origin: [f32; 3],
    pub extent: [f32; 3],
    pub sphere_radius: f32,
}

/// A single LOD level of a static mesh.
#[derive(Debug)]
pub struct LodModel {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub sections: Vec<MeshSection>,
    pub num_vertices: u32,
    pub num_triangles: u32,
}

/// A single vertex with position, normal, tangent, and UV data.
#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// Tangent vector; w component is the sign of the bitangent.
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}

/// A section of a mesh that uses a single material.
#[derive(Debug)]
pub struct MeshSection {
    pub material_index: i32,
    pub first_index: u32,
    pub num_triangles: u32,
    /// Package object reference for the material (negative = import).
    pub material_ref: i32,
}

impl StaticMesh {
    /// Build a flat list of collision triangles in mesh-local space.
    ///
    /// Returns one `[[f32; 3]; 3]` per triangle. The data source preference
    /// is:
    ///
    /// 1. **kDOP triangles** — the collision-specific subset the engine
    ///    uses for trace queries. Preferred for navmesh extraction because
    ///    the index list is smaller and already filtered to colliding
    ///    surfaces.
    /// 2. **LOD0 index buffer** — every render triangle in the highest
    ///    detail LOD. Used when the kDOPTree is empty (uncommon — usually
    ///    only true for editor-only meshes).
    ///
    /// All vertex indices that fall outside the LOD0 vertex range are
    /// silently dropped — a malformed mesh shouldn't kill the extractor.
    /// The number of dropped indices is logged at `warn` level so a
    /// downstream investigator can tell when this happened.
    pub fn collision_triangles(&self) -> Vec<[[f32; 3]; 3]> {
        let Some(lod0) = self.lod_models.first() else {
            return Vec::new();
        };
        let n = lod0.vertices.len() as u32;

        if !self.kdop_triangles.is_empty() {
            let mut out = Vec::with_capacity(self.kdop_triangles.len());
            let mut dropped = 0u32;
            for t in &self.kdop_triangles {
                let (v0, v1, v2) = (t.v0 as u32, t.v1 as u32, t.v2 as u32);
                if v0 >= n || v1 >= n || v2 >= n {
                    dropped += 1;
                    continue;
                }
                out.push([
                    lod0.vertices[v0 as usize].position,
                    lod0.vertices[v1 as usize].position,
                    lod0.vertices[v2 as usize].position,
                ]);
            }
            if dropped > 0 {
                tracing::warn!(
                    dropped,
                    kept = out.len(),
                    vertex_count = n,
                    "kDOP triangle index out of range; dropped"
                );
            }
            return out;
        }

        // Fall back to LOD0 index buffer. The index buffer is a flat
        // u16/u32 list; every three consecutive entries form a triangle.
        let mut out = Vec::with_capacity(lod0.indices.len() / 3);
        let mut dropped = 0u32;
        for triplet in lod0.indices.chunks_exact(3) {
            let (i0, i1, i2) = (triplet[0], triplet[1], triplet[2]);
            if i0 >= n || i1 >= n || i2 >= n {
                dropped += 1;
                continue;
            }
            out.push([
                lod0.vertices[i0 as usize].position,
                lod0.vertices[i1 as usize].position,
                lod0.vertices[i2 as usize].position,
            ]);
        }
        if dropped > 0 {
            tracing::warn!(
                dropped,
                kept = out.len(),
                vertex_count = n,
                "LOD0 triangle index out of range; dropped"
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collision_triangles_prefers_kdop_when_present() {
        // Build a 4-vertex mesh with both kDOP triangles and a LOD0 index
        // buffer that disagree. Caller must see the kDOP-derived list.
        let lod0 = LodModel {
            vertices: vec![
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
                Vertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
            ],
            // LOD0 says triangle (0, 1, 3) — would be the fallback.
            indices: vec![0, 1, 3],
            sections: vec![],
            num_vertices: 4,
            num_triangles: 1,
        };
        let mesh = StaticMesh {
            bounds: BoundingBox {
                origin: [0.0; 3],
                extent: [1.0; 3],
                sphere_radius: 1.0,
            },
            lod_models: vec![lod0],
            internal_version: 15,
            // kDOP says triangle (0, 2, 3) — distinct from the LOD0 fallback.
            kdop_triangles: vec![KdopTriangle {
                v0: 0,
                v1: 2,
                v2: 3,
                material: 0,
            }],
        };

        let tris = mesh.collision_triangles();
        assert_eq!(tris.len(), 1);
        assert_eq!(tris[0][0], [0.0, 0.0, 0.0]);
        assert_eq!(tris[0][1], [0.0, 1.0, 0.0]);
        assert_eq!(tris[0][2], [1.0, 1.0, 0.0]);
    }

    #[test]
    fn collision_triangles_falls_back_to_lod0_when_kdop_empty() {
        let lod0 = LodModel {
            vertices: vec![
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
            ],
            // Two triangles back-to-back; the second is degenerate but
            // still emitted — the extractor is not responsible for
            // pruning degeneracies, Recast does that.
            indices: vec![0, 1, 2, 0, 1, 2],
            sections: vec![],
            num_vertices: 3,
            num_triangles: 2,
        };
        let mesh = StaticMesh {
            bounds: BoundingBox {
                origin: [0.0; 3],
                extent: [1.0; 3],
                sphere_radius: 1.0,
            },
            lod_models: vec![lod0],
            internal_version: 15,
            kdop_triangles: vec![],
        };
        let tris = mesh.collision_triangles();
        assert_eq!(tris.len(), 2);
        assert_eq!(tris[0][0], [0.0, 0.0, 0.0]);
        assert_eq!(tris[0][1], [1.0, 0.0, 0.0]);
        assert_eq!(tris[0][2], [0.0, 1.0, 0.0]);
    }

    #[test]
    fn collision_triangles_drops_out_of_range_indices() {
        // One in-range kDOP triangle and one referencing a non-existent
        // vertex index 99 — the bad triangle must be dropped, the good
        // one must survive. This guards against the static_mesh decoder
        // returning a malformed kDOP list that would otherwise panic in
        // the indexer.
        let lod0 = LodModel {
            vertices: vec![
                Vertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
                Vertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
                Vertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0; 3],
                    tangent: [0.0; 4],
                    uv: [0.0; 2],
                },
            ],
            indices: vec![],
            sections: vec![],
            num_vertices: 3,
            num_triangles: 0,
        };
        let mesh = StaticMesh {
            bounds: BoundingBox {
                origin: [0.0; 3],
                extent: [1.0; 3],
                sphere_radius: 1.0,
            },
            lod_models: vec![lod0],
            internal_version: 15,
            kdop_triangles: vec![
                KdopTriangle {
                    v0: 0,
                    v1: 1,
                    v2: 2,
                    material: 0,
                },
                KdopTriangle {
                    v0: 0,
                    v1: 1,
                    v2: 99,
                    material: 0,
                },
            ],
        };
        let tris = mesh.collision_triangles();
        assert_eq!(tris.len(), 1);
    }
}

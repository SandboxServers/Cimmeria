//! `StaticMesh` export bodies.
//!
//! Only as much of the layout as
//! `cimmeria_upk_objects::deserialize_static_mesh` reads: bounds, a
//! `BodySetup` reference, an empty kDOP node array plus the kDOP
//! collision triangle list, `InternalVersion`, and one LOD carrying an
//! empty `RawTriangles` bulk-data header, no elements, a 40-byte-stride
//! vertex buffer and an index buffer.
//!
//! `StaticMesh::collision_triangles` prefers the kDOP list when it is
//! non-empty and falls back to the LOD0 index buffer otherwise, so a
//! fixture can drive either path by leaving one of the two empty.
//!
//! Unlike `Model`, this decoder does **not** enforce exact
//! consumption; the trailing per-LOD fields are simply not read.

/// Vertex stride the SGW cook uses (position, tangents, colour, UV).
const VERTEX_STRIDE: usize = 40;

/// A synthetic static mesh.
#[derive(Debug, Clone, Default)]
pub struct StaticMeshPayload {
    /// LOD0 vertex positions. Everything else in the 40-byte vertex is
    /// zeroed.
    pub positions: Vec<[f32; 3]>,
    /// kDOP collision triangles as `(v0, v1, v2)` indices into
    /// `positions`.
    pub kdop_triangles: Vec<(u16, u16, u16)>,
    /// LOD0 index buffer, used only when `kdop_triangles` is empty.
    pub indices: Vec<u16>,
}

impl StaticMeshPayload {
    /// A unit right triangle in the XY plane with one kDOP collision
    /// triangle over it.
    pub fn unit_triangle() -> Self {
        Self {
            positions: vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0]],
            kdop_triangles: vec![(0, 1, 2)],
            indices: Vec::new(),
        }
    }

    /// Encode the export body.
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0i32.to_le_bytes()); // NetIndex
        b.extend_from_slice(&0i32.to_le_bytes()); // FName "None" index
        b.extend_from_slice(&0i32.to_le_bytes()); // FName instance number

        // FBoxSphereBounds: origin, extent, radius.
        for f in [0.0f32, 0.0, 0.0, 100.0, 100.0, 100.0, 175.0] {
            b.extend_from_slice(&f.to_le_bytes());
        }
        b.extend_from_slice(&0i32.to_le_bytes()); // BodySetup objref

        b.extend_from_slice(&0i32.to_le_bytes()); // kDOP node count
        b.extend_from_slice(&(self.kdop_triangles.len() as i32).to_le_bytes());
        for (v0, v1, v2) in &self.kdop_triangles {
            for v in [*v0, *v1, *v2, 0u16] {
                b.extend_from_slice(&v.to_le_bytes());
            }
        }

        b.extend_from_slice(&15i32.to_le_bytes()); // InternalVersion
        b.extend_from_slice(&1i32.to_le_bytes()); // LODModels count

        // RawTriangles: an FUntypedBulkData v486 header with no payload.
        for v in [0i32; 4] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&0i32.to_le_bytes()); // Elements count
        b.extend_from_slice(&1i32.to_le_bytes()); // bUseFullPrecisionUVs

        b.extend_from_slice(&(VERTEX_STRIDE as i32).to_le_bytes());
        b.extend_from_slice(&(self.positions.len() as i32).to_le_bytes());
        b.extend_from_slice(&(self.positions.len() as i32).to_le_bytes());
        for p in &self.positions {
            let mut v = vec![0u8; VERTEX_STRIDE];
            for (k, c) in p.iter().enumerate() {
                v[k * 4..k * 4 + 4].copy_from_slice(&c.to_le_bytes());
            }
            b.extend_from_slice(&v);
        }

        b.extend_from_slice(&(self.positions.len() as i32).to_le_bytes());
        b.extend_from_slice(&(self.indices.len() as u32).to_le_bytes());
        for i in &self.indices {
            b.extend_from_slice(&i.to_le_bytes());
        }

        b.extend_from_slice(&0i32.to_le_bytes()); // Edges header
        b.extend_from_slice(&0i32.to_le_bytes()); // Edges count
        b
    }
}

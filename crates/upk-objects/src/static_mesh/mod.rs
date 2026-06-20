//! StaticMesh deserializer for UE3 export data (SGW v486).
//!
//! Binary layout after tagged properties (verified from AN-Antenna00 hex dump):
//!
//! ```text
//! 1. FBoxSphereBounds (28 bytes): origin[3f], extent[3f], radius[f]
//! 2. BodySetup object reference (i32)
//! 3. kDOPTree collision:
//!    - Node count (i32) + nodes (count * 32 bytes: 6 floats bbox + 2 u32 children)
//!    - Triangle count (i32) + triangles (count * 8 bytes: 3 u16 verts + 1 u16 material)
//! 4. InternalVersion (i32, typically 15)
//! 5. LODModels count (i32)
//! 6. Per LOD:
//!    a. RawTriangles: FUntypedBulkData v486 (16-byte header, empty in cooked packages)
//!    b. Elements: count (i32) + count * 28-byte FStaticMeshElement structs (7 x i32)
//!    c. VertexBuffer: stride (i32) + num_vertices (i32) + bulk_count (i32) +
//!       bulk_count * stride bytes of inline vertex data
//!    d. NumVertices (i32) + IndexCount (i32) + IndexCount * 2 bytes of u16 indices
//!    e. Edges: header (i32) + count (i32) + count * 16-byte FMeshEdge structs
//!    f. Trailing fields (skipped)
//! ```
//!
//! Vertex format (40 bytes per vertex, full-precision UVs):
//!   [+0]  Position:   3 x f32 (12 bytes)
//!   [+12] TangentX W: f32 (bitangent sign: 0.0 or 1.0)
//!   [+16] TangentX:   FPackedNormal (4 bytes)
//!   [+20] TangentY:   FPackedNormal (4 bytes)
//!   [+24] TangentZ:   FPackedNormal (4 bytes)
//!   [+28] Color:      u32 RGBA (4 bytes)
//!   [+32] UV:         2 x f32 (8 bytes)
//!
//! The module is split along two seams:
//! - [`types`] — the public decoded structs plus the `collision_triangles`
//!   accessor consumed by the navmesh extractor.
//! - [`parse`] — the byte-level deserializer and its size constants.

mod parse;
mod types;

pub use parse::deserialize_static_mesh;
pub use types::{BoundingBox, KdopTriangle, LodModel, MeshSection, StaticMesh, Vertex};

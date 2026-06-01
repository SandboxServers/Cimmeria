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

use byteorder::{ByteOrder, LittleEndian};

use crate::bulk_data::parse_bulk_data_v486;
use crate::error::{ObjectError, Result};

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

// --- kDOPTree element sizes (verified from hex analysis) ---

/// kDOPNode: 6 floats (bounding volume min/max XYZ) + 2 u32 (children/leaf indices).
const KDOP_NODE_SIZE: usize = 32;
/// kDOPCollisionTriangle: 3 u16 vertex indices + 1 u16 material index.
const KDOP_TRI_SIZE: usize = 8;
/// FMeshEdge: 2 i32 vertex indices + 2 i32 face indices.
const MESH_EDGE_SIZE: usize = 16;
/// Fields per FStaticMeshElement (serialized as 7 consecutive i32 values in SGW v486).
const ELEMENT_FIELD_COUNT: usize = 7;

/// Reject kDOPTree node arrays larger than this. The largest real
/// Castle_CellBlock kDOPTree fits in low five digits; this cap sits
/// comfortably above ground truth while still refusing a malicious file
/// from coercing a ~3GB allocation.
const MAX_KDOP_NODES: i32 = 100_000;

/// Reject kDOPTree triangle arrays larger than this. The largest real
/// Castle_CellBlock mesh has ~80k collision triangles; the cap sits an
/// order of magnitude above ground truth. Without it, a malformed file
/// declaring `tri_count = 0x7FFFFFFF` would coerce the parser into a
/// ~16GB allocation before the underlying read could fail.
const MAX_KDOP_TRIANGLES: i32 = 1_000_000;

/// Deserialize a StaticMesh from export serial data.
///
/// `data` is the raw bytes from `pkg.read_export_data(export)`.
/// `names` is the package name table for property parsing.
pub fn deserialize_static_mesh(
    data: &[u8],
    names: &[cimmeria_upk::NameEntry],
) -> Result<StaticMesh> {
    // 1. Parse tagged properties (start at offset 4, after NetIndex)
    let (_props, bin_offset) = cimmeria_upk::parse_tagged_properties_with_end(data, 4, names);

    let mut pos = bin_offset;

    // 2. FBoxSphereBounds (28 bytes)
    let bounds = read_bounds(data, &mut pos)?;

    // 3. BodySetup object reference (i32) -- skip
    ensure_bytes(data, pos, 4, "BodySetup")?;
    let _body_setup_ref = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    // 4. kDOPTree -- read collision triangles, skip nodes
    let kdop_triangles = read_kdop_tree(data, &mut pos)?;

    // 5. InternalVersion
    ensure_bytes(data, pos, 4, "InternalVersion")?;
    let internal_version = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    // 6. LODModels count
    ensure_bytes(data, pos, 4, "LODModels count")?;
    let lod_count = LittleEndian::read_i32(&data[pos..]);
    pos += 4;
    if !(0..=16).contains(&lod_count) {
        return Err(ObjectError::InvalidData(format!(
            "Unreasonable LOD count: {}",
            lod_count
        )));
    }

    // 7. Parse each LOD
    let mut lod_models = Vec::with_capacity(lod_count as usize);
    for lod_idx in 0..lod_count as usize {
        let lod = parse_lod(data, &mut pos, lod_idx)?;
        lod_models.push(lod);
    }

    // 8. Validate kDOP triangle indices against the LOD0 vertex count.
    validate_kdop_indices(
        &kdop_triangles,
        lod_models.first().map(|l| l.vertices.len()),
    )?;

    Ok(StaticMesh {
        bounds,
        lod_models,
        internal_version,
        kdop_triangles,
    })
}

/// Reject the mesh if any kDOP triangle references a vertex index that
/// falls outside the LOD0 vertex range.
///
/// The kDOP wire format stores vertex indices as `u16`, but a UE3
/// StaticMesh's LOD0 can in principle hold more than 65535 vertices.
/// When that happens, any kDOP triangle for a high-index vertex had its
/// wire value silently wrapped at cook time — there's no way to recover
/// the original index after the fact. The pragmatic choice is to fail
/// fast at parse so the navmesh extractor doesn't see the same triangle
/// later via `collision_triangles()` (where it would get silently dropped
/// and leave a hole in the navmesh).
///
/// Lifted into its own function so the regression test can exercise the
/// guard without constructing a full StaticMesh binary fixture.
fn validate_kdop_indices(
    kdop_triangles: &[KdopTriangle],
    lod0_vert_count: Option<usize>,
) -> Result<()> {
    let Some(vert_count) = lod0_vert_count else {
        // No LOD0 means no vertex buffer to validate against. The
        // mesh is unusable downstream regardless; let it through so the
        // caller's "no LOD models" path is the one that fires.
        return Ok(());
    };
    if let Some(bad) = kdop_triangles.iter().find(|t| {
        (t.v0 as usize) >= vert_count
            || (t.v1 as usize) >= vert_count
            || (t.v2 as usize) >= vert_count
    }) {
        return Err(ObjectError::InvalidData(format!(
            "kDOP triangle ({}, {}, {}) references vertex index >= LOD0 vertex count {} \
             — likely a u16 index wrap on a >65535-vertex mesh",
            bad.v0, bad.v1, bad.v2, vert_count
        )));
    }
    Ok(())
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

/// Read FBoxSphereBounds: 3 floats origin, 3 floats extent, 1 float radius.
fn read_bounds(data: &[u8], pos: &mut usize) -> Result<BoundingBox> {
    ensure_bytes(data, *pos, 28, "FBoxSphereBounds")?;
    let origin = [
        LittleEndian::read_f32(&data[*pos..]),
        LittleEndian::read_f32(&data[*pos + 4..]),
        LittleEndian::read_f32(&data[*pos + 8..]),
    ];
    let extent = [
        LittleEndian::read_f32(&data[*pos + 12..]),
        LittleEndian::read_f32(&data[*pos + 16..]),
        LittleEndian::read_f32(&data[*pos + 20..]),
    ];
    let sphere_radius = LittleEndian::read_f32(&data[*pos + 24..]);
    *pos += 28;
    Ok(BoundingBox {
        origin,
        extent,
        sphere_radius,
    })
}

/// Read the kDOPTree: skip the node array, return the parsed triangle list.
///
/// The kDOPTree on-disk layout is two TArrays:
///
/// 1. **Nodes** — `count + count * 32 bytes`. Each node is six floats
///    (axis-aligned bbox min/max) plus two u32 child/leaf indices. The
///    bounding-volume tree is rebuilt on load by the engine; we have no
///    use for the binary tree itself, so we just skip the bytes.
/// 2. **Triangles** — `count + count * 8 bytes`. Each triangle is three
///    u16 vertex indices (into the LOD0 vertex buffer) plus one u16
///    material/section index. **This is the collision-relevant data**
///    and we return it to the caller.
fn read_kdop_tree(data: &[u8], pos: &mut usize) -> Result<Vec<KdopTriangle>> {
    // kDOP nodes: count + count * 32 bytes. Bounds-check before any
    // Vec::with_capacity-like allocation — a malicious file can declare
    // `node_count = 0x7FFFFFFF` and force ~3GB of allocation upfront.
    ensure_bytes(data, *pos, 4, "kDOP node count")?;
    let node_count = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;
    if !(0..=MAX_KDOP_NODES).contains(&node_count) {
        return Err(ObjectError::InvalidData(format!(
            "Unreasonable kDOP node count: {} (max {})",
            node_count, MAX_KDOP_NODES
        )));
    }
    let node_data = node_count as usize * KDOP_NODE_SIZE;
    ensure_bytes(data, *pos, node_data, "kDOP node data")?;
    *pos += node_data;

    // kDOP triangles: count + count * 8 bytes. Same allocation-bomb
    // concern as the node array above — cap before we hit
    // Vec::with_capacity.
    ensure_bytes(data, *pos, 4, "kDOP triangle count")?;
    let tri_count = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;
    if !(0..=MAX_KDOP_TRIANGLES).contains(&tri_count) {
        return Err(ObjectError::InvalidData(format!(
            "Unreasonable kDOP triangle count: {} (max {})",
            tri_count, MAX_KDOP_TRIANGLES
        )));
    }
    let tri_data = tri_count as usize * KDOP_TRI_SIZE;
    ensure_bytes(data, *pos, tri_data, "kDOP triangle data")?;

    let mut triangles = Vec::with_capacity(tri_count as usize);
    for i in 0..tri_count as usize {
        let base = *pos + i * KDOP_TRI_SIZE;
        triangles.push(KdopTriangle {
            v0: LittleEndian::read_u16(&data[base..]),
            v1: LittleEndian::read_u16(&data[base + 2..]),
            v2: LittleEndian::read_u16(&data[base + 4..]),
            material: LittleEndian::read_u16(&data[base + 6..]),
        });
    }
    *pos += tri_data;

    tracing::trace!(
        "Read kDOPTree: {} nodes ({} bytes skipped) + {} triangles",
        node_count,
        node_data,
        tri_count,
    );
    Ok(triangles)
}

/// Parse a single LOD model.
fn parse_lod(data: &[u8], pos: &mut usize, lod_idx: usize) -> Result<LodModel> {
    // a. RawTriangles: FUntypedBulkData v486 header (empty in cooked packages)
    let raw_tris = parse_bulk_data_v486(data, *pos)?;
    *pos += raw_tris.bytes_consumed;
    // Raw triangle bulk data contains the original mesh data before cooking.
    // In shipped cooked packages this is always empty (count=0, size=0).
    if !raw_tris.data.is_empty() {
        tracing::debug!(
            "LOD {}: RawTriangles bulk data has {} bytes (unusual for cooked)",
            lod_idx,
            raw_tris.data.len()
        );
    }

    // b. Elements array: count + count * 32-byte FStaticMeshElement
    ensure_bytes(data, *pos, 4, "Elements count")?;
    let elem_count = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;
    if !(0..=256).contains(&elem_count) {
        return Err(ObjectError::InvalidData(format!(
            "LOD {}: unreasonable element count: {}",
            lod_idx, elem_count
        )));
    }

    let mut sections = Vec::with_capacity(elem_count as usize);
    for _ in 0..elem_count as usize {
        let section = parse_mesh_element(data, pos)?;
        sections.push(section);
    }

    // bUseFullPrecisionUVs flag (shared for all elements, after the array)
    ensure_bytes(data, *pos, 4, "bUseFullPrecisionUVs")?;
    let _use_full_precision_uvs = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;

    // c. VertexBuffer: stride (i32) + num_vertices (i32) + bulk_count (i32) + inline data
    ensure_bytes(data, *pos, 12, "VertexBuffer header")?;
    let stride = LittleEndian::read_i32(&data[*pos..]) as usize;
    let num_vertices_hdr = LittleEndian::read_i32(&data[*pos + 4..]) as u32;
    let bulk_count = LittleEndian::read_i32(&data[*pos + 8..]) as usize;
    *pos += 12;

    if stride == 0 || stride > 256 {
        return Err(ObjectError::InvalidData(format!(
            "LOD {}: unreasonable vertex stride: {}",
            lod_idx, stride
        )));
    }

    let vert_data_size = bulk_count * stride;
    ensure_bytes(data, *pos, vert_data_size, "vertex data")?;
    let vert_data = &data[*pos..*pos + vert_data_size];
    *pos += vert_data_size;

    tracing::debug!(
        "LOD {}: {} vertices, stride={}, data={} bytes",
        lod_idx,
        bulk_count,
        stride,
        vert_data_size
    );

    // Parse vertices from the inline data
    let vertices = parse_vertices(vert_data, bulk_count, stride)?;

    // d. Index buffer: NumVertices (i32) + IndexCount (i32) + IndexCount * 2 bytes (u16)
    ensure_bytes(data, *pos, 8, "index buffer header")?;
    let _num_vertices_2 = LittleEndian::read_u32(&data[*pos..]);
    let index_count = LittleEndian::read_u32(&data[*pos + 4..]);
    *pos += 8;

    let idx_data_size = index_count as usize * 2;
    ensure_bytes(data, *pos, idx_data_size, "index data")?;

    let mut indices = Vec::with_capacity(index_count as usize);
    for i in 0..index_count as usize {
        let idx = LittleEndian::read_u16(&data[*pos + i * 2..]);
        indices.push(idx as u32);
    }
    *pos += idx_data_size;

    tracing::debug!(
        "LOD {}: {} indices ({} triangles)",
        lod_idx,
        index_count,
        index_count / 3
    );

    // e. Edges: header (i32) + count (i32) + count * 16 bytes
    //    Skip entirely -- not needed for rendering.
    if *pos + 8 <= data.len() {
        let _edge_header = LittleEndian::read_i32(&data[*pos..]);
        let edge_count = LittleEndian::read_i32(&data[*pos + 4..]);
        *pos += 8;
        if (0..10_000_000).contains(&edge_count) {
            let edge_data = edge_count as usize * MESH_EDGE_SIZE;
            if *pos + edge_data <= data.len() {
                *pos += edge_data;
                tracing::trace!("LOD {}: skipped {} edges", lod_idx, edge_count);
            }
        }
    }

    // f. Trailing fields -- skip whatever remains for this LOD.
    //    For single-LOD meshes, this includes ShadowTriangleDoubleSided and LODInfo.
    //    We don't parse these; the caller gets vertices + indices + sections.

    // Compute num_triangles from the sections or index count
    let num_triangles = sections.iter().map(|s| s.num_triangles).sum::<u32>();
    let num_triangles = if num_triangles > 0 {
        num_triangles
    } else {
        index_count / 3
    };

    Ok(LodModel {
        vertices,
        indices,
        sections,
        num_vertices: num_vertices_hdr,
        num_triangles,
    })
}

/// Parse FStaticMeshElement: 7 consecutive i32 fields (28 bytes) in SGW v486.
///
/// Field layout (verified by cross-referencing 1-section and 2-section meshes):
///   [0] Material:          i32 (package object reference, negative = import)
///   [1] EnableCollision:   UBOOL (i32)
///   [2] OldEnableCollision: UBOOL (i32)
///   [3] FirstIndex:        i32
///   [4] NumTriangles:      i32
///   [5] MinVertexIndex:    i32
///   [6] MaxVertexIndex:    i32
fn parse_mesh_element(data: &[u8], pos: &mut usize) -> Result<MeshSection> {
    let size = ELEMENT_FIELD_COUNT * 4;
    ensure_bytes(data, *pos, size, "FStaticMeshElement")?;

    let material_ref = LittleEndian::read_i32(&data[*pos..]);
    // [1], [2] = EnableCollision, OldEnableCollision (skip)
    let first_index = LittleEndian::read_u32(&data[*pos + 12..]);
    let num_triangles = LittleEndian::read_u32(&data[*pos + 16..]);
    let _min_vertex = LittleEndian::read_i32(&data[*pos + 20..]);
    let _max_vertex = LittleEndian::read_i32(&data[*pos + 24..]);
    *pos += size;

    Ok(MeshSection {
        material_index: 0, // v486 doesn't store a separate material index
        first_index,
        num_triangles,
        material_ref,
    })
}

/// Parse vertices from raw vertex buffer data.
///
/// Handles the 40-byte combined vertex format (position + normals + UV).
/// Falls back to position-only extraction for unknown strides.
fn parse_vertices(data: &[u8], count: usize, stride: usize) -> Result<Vec<Vertex>> {
    let mut vertices = Vec::with_capacity(count);

    for i in 0..count {
        let base = i * stride;
        if base + stride > data.len() {
            return Err(ObjectError::InvalidData(format!(
                "Vertex {} at offset {} exceeds data length {}",
                i,
                base,
                data.len()
            )));
        }

        // Position is always the first 12 bytes regardless of stride
        let px = LittleEndian::read_f32(&data[base..]);
        let py = LittleEndian::read_f32(&data[base + 4..]);
        let pz = LittleEndian::read_f32(&data[base + 8..]);

        let (normal, tangent, uv) = if stride >= 40 {
            // Full 40-byte format: position(12) + tangentW(4) + tangentX(4) +
            // tangentY(4) + tangentZ(4) + color(4) + UV(8)
            let tangent_w = LittleEndian::read_f32(&data[base + 12..]);

            let tangent_x = unpack_normal(&data[base + 16..base + 20]);
            let _tangent_y = unpack_normal(&data[base + 20..base + 24]);
            let tangent_z = unpack_normal(&data[base + 24..base + 28]);
            // data[base+28..base+32] is vertex color (skip)

            let u = LittleEndian::read_f32(&data[base + 32..]);
            let v = LittleEndian::read_f32(&data[base + 36..]);

            let normal = [tangent_z[0], tangent_z[1], tangent_z[2]];
            let tangent = [tangent_x[0], tangent_x[1], tangent_x[2], tangent_w];

            (normal, tangent, [u, v])
        } else if stride >= 12 {
            // Position-only or minimal format -- fill defaults
            ([0.0f32, 0.0, 1.0], [1.0f32, 0.0, 0.0, 1.0], [0.0f32, 0.0])
        } else {
            return Err(ObjectError::InvalidData(format!(
                "Vertex stride {} too small (minimum 12 for position)",
                stride
            )));
        };

        vertices.push(Vertex {
            position: [px, py, pz],
            normal,
            tangent,
            uv,
        });
    }

    Ok(vertices)
}

/// Unpack a UE3 FPackedNormal (4 bytes XYZW, each 0..255) to a unit-length [f32; 3].
///
/// UE3 packs each component as `(value * 127.5 + 127.5)` and stores as u8.
/// To unpack: `component = (byte - 127.5) / 127.5 = byte / 127.5 - 1.0`.
fn unpack_normal(bytes: &[u8]) -> [f32; 3] {
    [
        bytes[0] as f32 / 127.5 - 1.0,
        bytes[1] as f32 / 127.5 - 1.0,
        bytes[2] as f32 / 127.5 - 1.0,
    ]
}

/// Check that enough bytes remain for the next read.
fn ensure_bytes(data: &[u8], pos: usize, needed: usize, field: &str) -> Result<()> {
    if pos + needed > data.len() {
        Err(ObjectError::InvalidData(format!(
            "{} requires {} bytes at offset {}, but only {} available",
            field,
            needed,
            pos,
            data.len().saturating_sub(pos)
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_normal_center() {
        // 127 should map to approximately 0.0 (slightly negative due to integer mapping)
        let n = unpack_normal(&[127, 127, 127, 0]);
        assert!(n[0].abs() < 0.01);
        assert!(n[1].abs() < 0.01);
        assert!(n[2].abs() < 0.01);
    }

    #[test]
    fn unpack_normal_positive() {
        // 255 should map to approximately +1.0
        let n = unpack_normal(&[255, 127, 127, 0]);
        assert!((n[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn unpack_normal_negative() {
        // 0 should map to approximately -1.0
        let n = unpack_normal(&[0, 127, 127, 0]);
        assert!((n[0] + 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_bounds_roundtrip() {
        let mut buf = vec![0u8; 28];
        // origin
        LittleEndian::write_f32(&mut buf[0..], 1.0);
        LittleEndian::write_f32(&mut buf[4..], 2.0);
        LittleEndian::write_f32(&mut buf[8..], 3.0);
        // extent
        LittleEndian::write_f32(&mut buf[12..], 10.0);
        LittleEndian::write_f32(&mut buf[16..], 20.0);
        LittleEndian::write_f32(&mut buf[20..], 30.0);
        // radius
        LittleEndian::write_f32(&mut buf[24..], 37.4);

        let mut pos = 0;
        let bb = read_bounds(&buf, &mut pos).unwrap();
        assert_eq!(pos, 28);
        assert_eq!(bb.origin, [1.0, 2.0, 3.0]);
        assert_eq!(bb.extent, [10.0, 20.0, 30.0]);
        assert!((bb.sphere_radius - 37.4).abs() < 0.001);
    }

    #[test]
    fn read_empty_kdop_tree() {
        // 0 nodes, 0 triangles
        let mut buf = vec![0u8; 8];
        LittleEndian::write_i32(&mut buf[0..], 0);
        LittleEndian::write_i32(&mut buf[4..], 0);

        let mut pos = 0;
        let tris = read_kdop_tree(&buf, &mut pos).unwrap();
        assert_eq!(pos, 8);
        assert!(tris.is_empty());
    }

    #[test]
    fn read_small_kdop_tree() {
        // 2 nodes (64 bytes) + 3 triangles (24 bytes) = 96 bytes total
        let mut buf = vec![0u8; 4 + 64 + 4 + 24];
        LittleEndian::write_i32(&mut buf[0..], 2);
        LittleEndian::write_i32(&mut buf[68..], 3);
        // Triangle 0 starts at offset 72: indices (0, 1, 2) material 7
        LittleEndian::write_u16(&mut buf[72..], 0);
        LittleEndian::write_u16(&mut buf[74..], 1);
        LittleEndian::write_u16(&mut buf[76..], 2);
        LittleEndian::write_u16(&mut buf[78..], 7);
        // Triangle 1: (3, 4, 5) material 0
        LittleEndian::write_u16(&mut buf[80..], 3);
        LittleEndian::write_u16(&mut buf[82..], 4);
        LittleEndian::write_u16(&mut buf[84..], 5);
        // Triangle 2: (6, 7, 8) material 0
        LittleEndian::write_u16(&mut buf[88..], 6);
        LittleEndian::write_u16(&mut buf[90..], 7);
        LittleEndian::write_u16(&mut buf[92..], 8);

        let mut pos = 0;
        let tris = read_kdop_tree(&buf, &mut pos).unwrap();
        assert_eq!(pos, 96);
        assert_eq!(tris.len(), 3);
        assert_eq!(tris[0].v0, 0);
        assert_eq!(tris[0].v1, 1);
        assert_eq!(tris[0].v2, 2);
        assert_eq!(tris[0].material, 7);
        assert_eq!(tris[1].v0, 3);
        assert_eq!(tris[2].v2, 8);
    }

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

    #[test]
    fn read_kdop_tree_rejects_oversized_tri_count() {
        // Regression guard for the allocation-bomb class of bug: a
        // malicious file that declares `tri_count = i32::MAX` must be
        // rejected before the parser hits `Vec::with_capacity(...)`,
        // not after a ~16GB allocation attempt.
        //
        // Encode: 0 nodes, then `tri_count = 0x7FFFFFFF`. We don't bother
        // including the (impossible) triangle payload — the rejection
        // must happen before any of those bytes are read.
        let mut buf = vec![0u8; 8];
        LittleEndian::write_i32(&mut buf[0..], 0); // 0 kDOP nodes
        LittleEndian::write_i32(&mut buf[4..], i32::MAX); // malicious tri_count

        let mut pos = 0;
        let result = read_kdop_tree(&buf, &mut pos);
        let err = result.expect_err("oversized tri_count must error, not panic");
        let msg = format!("{err}");
        assert!(
            msg.contains("Unreasonable kDOP triangle count"),
            "error must call out the bounds-check failure; got: {msg}"
        );
    }

    #[test]
    fn read_kdop_tree_rejects_oversized_node_count() {
        // Companion guard for the kDOP node array — same allocation-bomb
        // shape, different field. Catches a malicious file that uses the
        // node-array side to drive the parser into a multi-GB allocation.
        let mut buf = vec![0u8; 4];
        LittleEndian::write_i32(&mut buf[0..], i32::MAX);
        let mut pos = 0;
        let result = read_kdop_tree(&buf, &mut pos);
        let err = result.expect_err("oversized node_count must error");
        assert!(format!("{err}").contains("Unreasonable kDOP node count"));
    }

    #[test]
    fn read_kdop_tree_rejects_negative_tri_count() {
        // Negative counts can't survive a `Vec::with_capacity` cast to
        // usize cleanly — guard explicitly. Without the bounds check the
        // multiplication `tri_count as usize * KDOP_TRI_SIZE` overflows
        // and panics on debug builds.
        let mut buf = vec![0u8; 8];
        LittleEndian::write_i32(&mut buf[0..], 0);
        LittleEndian::write_i32(&mut buf[4..], -1);
        let mut pos = 0;
        let result = read_kdop_tree(&buf, &mut pos);
        assert!(result.is_err(), "negative tri_count must error");
    }

    #[test]
    fn validate_kdop_indices_passes_when_all_in_range() {
        let tris = vec![
            KdopTriangle {
                v0: 0,
                v1: 1,
                v2: 2,
                material: 0,
            },
            KdopTriangle {
                v0: 99,
                v1: 50,
                v2: 25,
                material: 0,
            },
        ];
        assert!(validate_kdop_indices(&tris, Some(100)).is_ok());
    }

    #[test]
    fn validate_kdop_indices_rejects_out_of_range() {
        // A mesh with 100 LOD0 verts can only legally reference indices
        // 0..=99. Index 100 is out of range and could be a u16 wrap from
        // a >65535-vertex mesh — fail fast at parse so the navmesh
        // extractor doesn't see the same triangle later and silently
        // drop it (leaving a hole in the navmesh).
        let tris = vec![
            KdopTriangle {
                v0: 0,
                v1: 1,
                v2: 2,
                material: 0,
            },
            KdopTriangle {
                v0: 0,
                v1: 1,
                v2: 100,
                material: 0,
            },
        ];
        let err =
            validate_kdop_indices(&tris, Some(100)).expect_err("out-of-range index must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("u16 index wrap"),
            "error message must explain the u16-wrap class of failure; got: {msg}"
        );
    }

    #[test]
    fn validate_kdop_indices_rejects_u16_wrap_scenario() {
        // The literal scenario: a mesh with >65535 verts. The wire can't
        // represent index 70000 (>65535), so the cooker would either
        // have wrapped (silent corruption) or — if the cook chain is
        // sane — never emitted a kDOP for that mesh in the first place.
        // Either way, if we see an out-of-range index that could have
        // come from a u16 wrap, reject.
        let tris = vec![KdopTriangle {
            v0: 0,
            v1: 1,
            v2: 65535,
            material: 0,
        }];
        assert!(validate_kdop_indices(&tris, Some(100)).is_err());
    }

    #[test]
    fn validate_kdop_indices_with_no_lod_is_lenient() {
        // No LOD0 vertex buffer to validate against → pass through. The
        // caller's "no LOD models" path will surface the unusability.
        let tris = vec![KdopTriangle {
            v0: 99,
            v1: 99,
            v2: 99,
            material: 0,
        }];
        assert!(validate_kdop_indices(&tris, None).is_ok());
    }

    #[test]
    fn parse_single_vertex_40byte() {
        let stride = 40;
        let mut buf = vec![0u8; stride];
        // Position: (100.0, 200.0, 300.0)
        LittleEndian::write_f32(&mut buf[0..], 100.0);
        LittleEndian::write_f32(&mut buf[4..], 200.0);
        LittleEndian::write_f32(&mut buf[8..], 300.0);
        // TangentX W (bitangent sign)
        LittleEndian::write_f32(&mut buf[12..], 1.0);
        // TangentX packed: (255, 127, 127, 0) => (+1, 0, 0)
        buf[16] = 255;
        buf[17] = 127;
        buf[18] = 127;
        buf[19] = 0;
        // TangentY packed
        buf[20] = 127;
        buf[21] = 255;
        buf[22] = 127;
        buf[23] = 0;
        // TangentZ packed: (127, 127, 255, 0) => (0, 0, +1)
        buf[24] = 127;
        buf[25] = 127;
        buf[26] = 255;
        buf[27] = 0;
        // Color (white)
        buf[28] = 0xFF;
        buf[29] = 0xFF;
        buf[30] = 0xFF;
        buf[31] = 0xFF;
        // UV: (0.5, 0.75)
        LittleEndian::write_f32(&mut buf[32..], 0.5);
        LittleEndian::write_f32(&mut buf[36..], 0.75);

        let verts = parse_vertices(&buf, 1, stride).unwrap();
        assert_eq!(verts.len(), 1);
        let v = &verts[0];
        assert_eq!(v.position, [100.0, 200.0, 300.0]);
        assert!((v.tangent[3] - 1.0).abs() < 0.01);
        assert!((v.uv[0] - 0.5).abs() < 0.001);
        assert!((v.uv[1] - 0.75).abs() < 0.001);
        // Normal should be approximately (0, 0, 1) from TangentZ
        assert!(v.normal[2] > 0.9);
    }
}

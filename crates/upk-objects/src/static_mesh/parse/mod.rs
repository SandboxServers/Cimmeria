//! Binary deserializer for the UE3 StaticMesh export payload (SGW v486).
//!
//! See the [`crate::static_mesh`] module docs for the full on-disk layout.
//! This module owns the byte-level reader: bounds, the kDOPTree, per-LOD
//! vertex/index/element parsing, and the FPackedNormal unpacker.

use byteorder::{ByteOrder, LittleEndian};

use super::types::{BoundingBox, KdopTriangle, LodModel, MeshSection, StaticMesh, Vertex};
use crate::bulk_data::parse_bulk_data_v486;
use crate::error::{ObjectError, Result};

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
mod tests;

//! StaticMesh deserializer for UE3 export data (SGW v486).
//!
//! Binary layout after tagged properties (verified from AN-Antenna00 hex dump):
//!
//!   1. FBoxSphereBounds (28 bytes): origin[3f], extent[3f], radius[f]
//!   2. BodySetup object reference (i32)
//!   3. kDOPTree collision:
//!      - Node count (i32) + nodes (count * 32 bytes: 6 floats bbox + 2 u32 children)
//!      - Triangle count (i32) + triangles (count * 8 bytes: 3 u16 verts + 1 u16 material)
//!   4. InternalVersion (i32, typically 15)
//!   5. LODModels count (i32)
//!   6. Per LOD:
//!      a. RawTriangles: FUntypedBulkData v486 (16-byte header, empty in cooked packages)
//!      b. Elements: count (i32) + count * 28-byte FStaticMeshElement structs (7 x i32)
//!      c. VertexBuffer: stride (i32) + num_vertices (i32) + bulk_count (i32) +
//!         bulk_count * stride bytes of inline vertex data
//!      d. NumVertices (i32) + IndexCount (i32) + IndexCount * 2 bytes of u16 indices
//!      e. Edges: header (i32) + count (i32) + count * 16-byte FMeshEdge structs
//!      f. Trailing fields (skipped)
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

/// Deserialize a StaticMesh from export serial data.
///
/// `data` is the raw bytes from `pkg.read_export_data(export)`.
/// `names` is the package name table for property parsing.
pub fn deserialize_static_mesh(
    data: &[u8],
    names: &[cimmeria_upk::NameEntry],
) -> Result<StaticMesh> {
    // 1. Parse tagged properties (start at offset 4, after NetIndex)
    let (_props, bin_offset) =
        cimmeria_upk::parse_tagged_properties_with_end(data, 4, names);

    let mut pos = bin_offset;

    // 2. FBoxSphereBounds (28 bytes)
    let bounds = read_bounds(data, &mut pos)?;

    // 3. BodySetup object reference (i32) -- skip
    ensure_bytes(data, pos, 4, "BodySetup")?;
    let _body_setup_ref = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    // 4. kDOPTree -- skip entirely
    skip_kdop_tree(data, &mut pos)?;

    // 5. InternalVersion
    ensure_bytes(data, pos, 4, "InternalVersion")?;
    let internal_version = LittleEndian::read_i32(&data[pos..]);
    pos += 4;

    // 6. LODModels count
    ensure_bytes(data, pos, 4, "LODModels count")?;
    let lod_count = LittleEndian::read_i32(&data[pos..]);
    pos += 4;
    if lod_count < 0 || lod_count > 16 {
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

    Ok(StaticMesh {
        bounds,
        lod_models,
        internal_version,
    })
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

/// Skip the kDOPTree: two TArrays with known per-element sizes.
fn skip_kdop_tree(data: &[u8], pos: &mut usize) -> Result<()> {
    // kDOP nodes: count + count * 32 bytes
    ensure_bytes(data, *pos, 4, "kDOP node count")?;
    let node_count = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;
    if node_count < 0 || node_count > 100_000 {
        return Err(ObjectError::InvalidData(format!(
            "Unreasonable kDOP node count: {}",
            node_count
        )));
    }
    let node_data = node_count as usize * KDOP_NODE_SIZE;
    ensure_bytes(data, *pos, node_data, "kDOP node data")?;
    *pos += node_data;

    // kDOP triangles: count + count * 8 bytes
    ensure_bytes(data, *pos, 4, "kDOP triangle count")?;
    let tri_count = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;
    if tri_count < 0 || tri_count > 1_000_000 {
        return Err(ObjectError::InvalidData(format!(
            "Unreasonable kDOP triangle count: {}",
            tri_count
        )));
    }
    let tri_data = tri_count as usize * KDOP_TRI_SIZE;
    ensure_bytes(data, *pos, tri_data, "kDOP triangle data")?;
    *pos += tri_data;

    tracing::trace!(
        "Skipped kDOPTree: {} nodes ({} bytes) + {} triangles ({} bytes)",
        node_count,
        node_data,
        tri_count,
        tri_data
    );
    Ok(())
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
    if elem_count < 0 || elem_count > 256 {
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
        if edge_count >= 0 && edge_count < 10_000_000 {
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
                i, base, data.len()
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
    fn skip_empty_kdop_tree() {
        // 0 nodes, 0 triangles
        let mut buf = vec![0u8; 8];
        LittleEndian::write_i32(&mut buf[0..], 0);
        LittleEndian::write_i32(&mut buf[4..], 0);

        let mut pos = 0;
        skip_kdop_tree(&buf, &mut pos).unwrap();
        assert_eq!(pos, 8);
    }

    #[test]
    fn skip_small_kdop_tree() {
        // 2 nodes (64 bytes) + 3 triangles (24 bytes) = 96 bytes total
        let mut buf = vec![0u8; 4 + 64 + 4 + 24];
        LittleEndian::write_i32(&mut buf[0..], 2);
        LittleEndian::write_i32(&mut buf[68..], 3);

        let mut pos = 0;
        skip_kdop_tree(&buf, &mut pos).unwrap();
        assert_eq!(pos, 96);
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

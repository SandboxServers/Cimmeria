//! Byte-level deserializers for the UE3 `UModel` and `UPolys` export
//! payloads (SGW Epic ver 486).
//!
//! See the [`crate::model`] module docs for the full on-disk layout and
//! `docs/reverse-engineering/findings/bsp-model-polys-serialize.md` for
//! the Ghidra evidence behind each field.
//!
//! # Exactness contract
//!
//! Both entry points assert that the fields they declare consume the
//! export's serial data **exactly**. A short buffer errors on the
//! offending field; a buffer with bytes left over errors with the
//! remainder count. This is deliberate: every field after `Verts` is
//! skipped by declared size, so an off-by-one anywhere upstream only
//! shows up as a non-zero remainder. Downgrading that to a silent
//! truncation would let a mis-parsed `Nodes` array reach the navmesh as
//! plausible-looking garbage.

use byteorder::{ByteOrder, LittleEndian};

use super::types::{BspNode, BspSurf, BspVert, Model, ModelBounds, Poly, Polys};
use crate::error::{ObjectError, Result};

/// `FBoxSphereBounds`: Origin.xyz + BoxExtent.xyz + SphereRadius.
const MODEL_BOUNDS_SIZE: usize = 28;
/// `FVector` — 3 x f32.
const FVECTOR_SIZE: usize = 12;
/// `FBspNode` wire stride.
pub const FBSP_NODE_SIZE: usize = 68;
/// `FBspSurf` wire stride (ArVer > 0x1a0; SGW is 486).
pub const FBSP_SURF_SIZE: usize = 56;
/// `FVert` wire stride.
pub const FVERT_SIZE: usize = 24;
/// `FZoneProperties` wire stride — ZoneActor objref + LastRenderTime +
/// Connectivity QWORD + Visibility QWORD.
const FZONE_SIZE: usize = 24;
/// `Model::Zones` is a fixed C array, not a `TArray`; `NumZones` counts
/// how many of its 64 slots are on the wire.
const MAX_ZONES: i32 = 64;
/// Stride of the first unidentified trailing `TArray`. Structurally
/// confirmed across six real exports; contents never decoded.
const TRAILING_ARRAY_A_SIZE: usize = 16;
/// Stride of the second unidentified trailing `TArray` (count equals
/// the preceding "NumUniqueVertices"-shaped INT on the one large
/// sample checked). Likely a per-unique-vertex lighting cache.
const TRAILING_ARRAY_B_SIZE: usize = 40;

// --- FBspNode field offsets (see the finding's confidence table) ---
const NODE_OFF_PLANE: usize = 0x00;
const NODE_OFF_IVERTPOOL: usize = 0x18;
const NODE_OFF_ISURF: usize = 0x1c;
const NODE_OFF_NUMVERTICES: usize = 0x3a;
const NODE_OFF_NODEFLAGS: usize = 0x3b;

/// `UModel::Serialize`'s `IsLoading` fixup: the loader masks NodeFlags
/// down to its low five bits. We reproduce it so a decoded node's
/// `node_flags` matches what the running client would see — otherwise a
/// flag-based filter written against client behaviour would test bits
/// the client never observes.
const NODE_FLAGS_LOAD_MASK: u8 = 0x1f;

// --- FBspSurf field offsets (FBspSurf_Serialize, decompiled) ---
const SURF_OFF_MATERIAL: usize = 0x00;
const SURF_OFF_POLYFLAGS: usize = 0x04;
const SURF_OFF_PBASE: usize = 0x08;
const SURF_OFF_VNORMAL: usize = 0x0c;
const SURF_OFF_IBRUSHPOLY: usize = 0x18;
const SURF_OFF_ACTOR: usize = 0x1c;
const SURF_OFF_PLANE: usize = 0x20;

/// Deserialize a `UModel` from an export's serial data.
///
/// `data` is the raw bytes from `pkg.read_export_data(export)`;
/// `names` is the package name table.
///
/// Errors if the declared fields do not consume `data` exactly.
pub fn deserialize_model(data: &[u8], names: &[cimmeria_upk::NameEntry]) -> Result<Model> {
    // 4-byte NetIndex prefix, then the tagged-property stream. Empty
    // (just the `None` terminator) for every Model export sampled, but
    // parsed properly rather than assumed.
    let (_props, mut pos) = cimmeria_upk::parse_tagged_properties_with_end(data, 4, names);

    let bounds = read_bounds(data, &mut pos)?;
    let vectors = read_vectors(data, &mut pos, "Vectors")?;
    let points = read_vectors(data, &mut pos, "Points")?;
    let nodes = read_nodes(data, &mut pos)?;

    // ArVer > 0x140: a single object reference of unidentified purpose.
    // Always present for SGW (ArVer 486). Skipped, not interpreted.
    let _unidentified_objref = read_i32(data, &mut pos, "post-Nodes objref")?;

    let surfs = read_surfs(data, &mut pos)?;
    let verts = read_verts(data, &mut pos)?;

    let num_shared_sides = read_i32(data, &mut pos, "NumSharedSides")?;

    // Zones is a fixed Zones[64] C array; only NumZones entries are on
    // the wire. Not collision-relevant — skipped by declared size.
    let num_zones = read_i32(data, &mut pos, "NumZones")?;
    if !(0..=MAX_ZONES).contains(&num_zones) {
        return Err(ObjectError::InvalidData(format!(
            "Model: NumZones {num_zones} outside the fixed Zones[{MAX_ZONES}] array"
        )));
    }
    skip_bytes(data, &mut pos, num_zones as usize * FZONE_SIZE, "Zones")?;

    let polys_ref = read_i32(data, &mut pos, "Polys objref")?;

    skip_array(data, &mut pos, 4, "LeafHulls")?;
    skip_array(data, &mut pos, 4, "Leaves")?;

    let root_outside = read_i32(data, &mut pos, "RootOutside")? != 0;
    let linked = read_i32(data, &mut pos, "Linked")? != 0;

    skip_array(data, &mut pos, 4, "PortalNodes")?;
    skip_array(data, &mut pos, TRAILING_ARRAY_A_SIZE, "trailing array A")?;

    // ArVer >= 0x14d: an INT whose value equalled the following array's
    // element count on the one large sample walked — consistent with a
    // `NumUniqueVertices` sizing hint. Read and discarded; the array
    // that follows carries its own count.
    let _num_unique_vertices = read_i32(data, &mut pos, "NumUniqueVertices")?;
    skip_array(data, &mut pos, TRAILING_ARRAY_B_SIZE, "trailing array B")?;

    require_exact(pos, data.len(), "Model")?;

    Ok(Model {
        bounds,
        vectors,
        points,
        nodes,
        surfs,
        verts,
        num_shared_sides,
        num_zones,
        polys_ref,
        root_outside,
        linked,
    })
}

/// Deserialize a `UPolys` from an export's serial data.
///
/// The `Element` array header is **three** i32 values, not the two a
/// plain `TArray` would have: `Count`, a legacy `Max` the loader
/// immediately discards, and a legacy object reference read through the
/// archive's objref slot and also discarded. Both legacy fields are
/// vestiges of an older on-disk `TArray<T>` format.
pub fn deserialize_polys(data: &[u8], names: &[cimmeria_upk::NameEntry]) -> Result<Polys> {
    let (_props, mut pos) = cimmeria_upk::parse_tagged_properties_with_end(data, 4, names);

    let count = read_i32(data, &mut pos, "Polys::Element count")?;
    let _legacy_max = read_i32(data, &mut pos, "Polys::Element legacy Max")?;
    let _legacy_objref = read_i32(data, &mut pos, "Polys::Element legacy objref")?;
    if count < 0 {
        return Err(ObjectError::InvalidData(format!(
            "Polys: negative element count {count}"
        )));
    }
    // An FPoly is at minimum 88 bytes, so a count that can't fit in the
    // remaining buffer is rejected before any allocation.
    let remaining = data.len().saturating_sub(pos);
    if (count as usize).saturating_mul(88) > remaining {
        return Err(ObjectError::InvalidData(format!(
            "Polys: element count {count} needs at least {} bytes but only {remaining} remain",
            count as usize * 88
        )));
    }

    let mut elements = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        elements.push(read_poly(data, &mut pos, i)?);
    }

    require_exact(pos, data.len(), "Polys")?;
    Ok(Polys { elements })
}

/// Read one variable-length `FPoly` (`88 + 12 * NumVertices` bytes).
fn read_poly(data: &[u8], pos: &mut usize, index: usize) -> Result<Poly> {
    let base = read_fvector(data, pos, "FPoly::Base")?;
    let normal = read_fvector(data, pos, "FPoly::Normal")?;
    let texture_u = read_fvector(data, pos, "FPoly::TextureU")?;
    let texture_v = read_fvector(data, pos, "FPoly::TextureV")?;

    let vert_count = read_i32(data, pos, "FPoly::Vertices count")?;
    if vert_count < 0 {
        return Err(ObjectError::InvalidData(format!(
            "FPoly {index}: negative vertex count {vert_count}"
        )));
    }
    let vert_bytes = (vert_count as usize) * FVECTOR_SIZE;
    ensure_bytes(data, *pos, vert_bytes, "FPoly::Vertices")?;
    let mut vertices = Vec::with_capacity(vert_count as usize);
    for k in 0..vert_count as usize {
        let o = *pos + k * FVECTOR_SIZE;
        vertices.push([
            LittleEndian::read_f32(&data[o..]),
            LittleEndian::read_f32(&data[o + 4..]),
            LittleEndian::read_f32(&data[o + 8..]),
        ]);
    }
    *pos += vert_bytes;

    let poly_flags = read_i32(data, pos, "FPoly::PolyFlags")? as u32;
    let actor_ref = read_i32(data, pos, "FPoly::Actor")?;
    // ItemName is an FName: name-table index + instance number.
    skip_bytes(data, pos, 8, "FPoly::ItemName")?;
    let material_ref = read_i32(data, pos, "FPoly::Material")?;
    // Three unidentified trailing INTs plus one more gated on
    // ArVer > 0x1a1 (true for SGW). There is NO extra 4-byte gap at the
    // `+0x58`-relative offset the in-memory struct has — including one
    // misaligns every subsequent element in a multi-element export.
    skip_bytes(data, pos, 16, "FPoly trailing fields")?;

    Ok(Poly {
        base,
        normal,
        texture_u,
        texture_v,
        vertices,
        poly_flags,
        actor_ref,
        material_ref,
    })
}

fn read_bounds(data: &[u8], pos: &mut usize) -> Result<ModelBounds> {
    ensure_bytes(data, *pos, MODEL_BOUNDS_SIZE, "FBoxSphereBounds")?;
    let p = *pos;
    let bounds = ModelBounds {
        origin: [
            LittleEndian::read_f32(&data[p..]),
            LittleEndian::read_f32(&data[p + 4..]),
            LittleEndian::read_f32(&data[p + 8..]),
        ],
        box_extent: [
            LittleEndian::read_f32(&data[p + 12..]),
            LittleEndian::read_f32(&data[p + 16..]),
            LittleEndian::read_f32(&data[p + 20..]),
        ],
        sphere_radius: LittleEndian::read_f32(&data[p + 24..]),
    };
    *pos += MODEL_BOUNDS_SIZE;
    Ok(bounds)
}

fn read_vectors(data: &[u8], pos: &mut usize, field: &str) -> Result<Vec<[f32; 3]>> {
    let (start, count) = read_array_header(data, pos, FVECTOR_SIZE, field)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * FVECTOR_SIZE;
        out.push([
            LittleEndian::read_f32(&data[o..]),
            LittleEndian::read_f32(&data[o + 4..]),
            LittleEndian::read_f32(&data[o + 8..]),
        ]);
    }
    Ok(out)
}

fn read_nodes(data: &[u8], pos: &mut usize) -> Result<Vec<BspNode>> {
    let (start, count) = read_array_header(data, pos, FBSP_NODE_SIZE, "Nodes")?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * FBSP_NODE_SIZE;
        out.push(BspNode {
            plane: [
                LittleEndian::read_f32(&data[o + NODE_OFF_PLANE..]),
                LittleEndian::read_f32(&data[o + NODE_OFF_PLANE + 4..]),
                LittleEndian::read_f32(&data[o + NODE_OFF_PLANE + 8..]),
                LittleEndian::read_f32(&data[o + NODE_OFF_PLANE + 12..]),
            ],
            i_vert_pool: LittleEndian::read_i32(&data[o + NODE_OFF_IVERTPOOL..]),
            i_surf: LittleEndian::read_i32(&data[o + NODE_OFF_ISURF..]),
            num_vertices: data[o + NODE_OFF_NUMVERTICES],
            node_flags: data[o + NODE_OFF_NODEFLAGS] & NODE_FLAGS_LOAD_MASK,
        });
    }
    Ok(out)
}

fn read_surfs(data: &[u8], pos: &mut usize) -> Result<Vec<BspSurf>> {
    let (start, count) = read_array_header(data, pos, FBSP_SURF_SIZE, "Surfs")?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * FBSP_SURF_SIZE;
        out.push(BspSurf {
            material_ref: LittleEndian::read_i32(&data[o + SURF_OFF_MATERIAL..]),
            poly_flags: LittleEndian::read_u32(&data[o + SURF_OFF_POLYFLAGS..]),
            p_base: LittleEndian::read_i32(&data[o + SURF_OFF_PBASE..]),
            v_normal: LittleEndian::read_i32(&data[o + SURF_OFF_VNORMAL..]),
            i_brush_poly: LittleEndian::read_i32(&data[o + SURF_OFF_IBRUSHPOLY..]),
            actor_ref: LittleEndian::read_i32(&data[o + SURF_OFF_ACTOR..]),
            plane: [
                LittleEndian::read_f32(&data[o + SURF_OFF_PLANE..]),
                LittleEndian::read_f32(&data[o + SURF_OFF_PLANE + 4..]),
                LittleEndian::read_f32(&data[o + SURF_OFF_PLANE + 8..]),
                LittleEndian::read_f32(&data[o + SURF_OFF_PLANE + 12..]),
            ],
        });
    }
    Ok(out)
}

fn read_verts(data: &[u8], pos: &mut usize) -> Result<Vec<BspVert>> {
    let (start, count) = read_array_header(data, pos, FVERT_SIZE, "Verts")?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * FVERT_SIZE;
        out.push(BspVert {
            p_vertex: LittleEndian::read_i32(&data[o..]),
        });
    }
    Ok(out)
}

/// Read a `TArray` header, bounds-check the payload, advance `pos` past
/// it, and return `(payload_start, element_count)`.
///
/// The bounds check happens **before** the caller's
/// `Vec::with_capacity`, so a hostile count can't drive a multi-GB
/// allocation: `count * elem_size` is checked against the bytes
/// actually present. No arbitrary `MAX_*` cap is needed because the
/// buffer length is the natural bound.
fn read_array_header(
    data: &[u8],
    pos: &mut usize,
    elem_size: usize,
    field: &str,
) -> Result<(usize, usize)> {
    let count = read_i32(data, pos, field)?;
    if count < 0 {
        return Err(ObjectError::InvalidData(format!(
            "{field}: negative element count {count}"
        )));
    }
    let bytes = (count as usize).checked_mul(elem_size).ok_or_else(|| {
        ObjectError::InvalidData(format!("{field}: element count {count} overflows"))
    })?;
    ensure_bytes(data, *pos, bytes, field)?;
    let start = *pos;
    *pos += bytes;
    Ok((start, count as usize))
}

/// Read a `TArray` header and skip the payload without decoding it.
fn skip_array(data: &[u8], pos: &mut usize, elem_size: usize, field: &str) -> Result<()> {
    read_array_header(data, pos, elem_size, field).map(|_| ())
}

fn read_i32(data: &[u8], pos: &mut usize, field: &str) -> Result<i32> {
    ensure_bytes(data, *pos, 4, field)?;
    let v = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;
    Ok(v)
}

fn read_fvector(data: &[u8], pos: &mut usize, field: &str) -> Result<[f32; 3]> {
    ensure_bytes(data, *pos, FVECTOR_SIZE, field)?;
    let v = [
        LittleEndian::read_f32(&data[*pos..]),
        LittleEndian::read_f32(&data[*pos + 4..]),
        LittleEndian::read_f32(&data[*pos + 8..]),
    ];
    *pos += FVECTOR_SIZE;
    Ok(v)
}

fn skip_bytes(data: &[u8], pos: &mut usize, n: usize, field: &str) -> Result<()> {
    ensure_bytes(data, *pos, n, field)?;
    *pos += n;
    Ok(())
}

fn ensure_bytes(data: &[u8], pos: usize, needed: usize, field: &str) -> Result<()> {
    if pos.saturating_add(needed) > data.len() {
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

/// Enforce the exactness contract: the declared fields must land on the
/// last byte of the export.
fn require_exact(pos: usize, len: usize, what: &str) -> Result<()> {
    if pos != len {
        return Err(ObjectError::InvalidData(format!(
            "{what} export not consumed exactly: stopped at offset {pos} of {len} \
             ({} bytes remaining) — field layout is wrong, refusing to truncate silently",
            len.saturating_sub(pos)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

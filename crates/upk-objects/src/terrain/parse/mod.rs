//! Binary deserializer for the UE3 `Terrain` export payload (SGW v486).
//!
//! See the [`crate::terrain`] module docs for the on-disk layout and the
//! provenance of the recipe.

use byteorder::{ByteOrder, LittleEndian};
use cimmeria_upk::{NameEntry, PropValue, TaggedProperty};

use super::types::{Terrain, SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D};
use crate::error::{ObjectError, Result};

/// `AActor` subclasses carry a 32-byte native header before their
/// tagged-property stream (4-byte NetIndex for plain objects, 8 for
/// components). `Terrain` is an actor.
const ACTOR_HEADER_SIZE: usize = 32;

/// Reject grids wider than this on either axis before allocating.
///
/// The largest shipped SGW terrain is 101×101 vertices (Castle). The cap
/// bounds the heightmap at `4096 * 4096 * 2 B = 32 MB` so a malformed
/// `NumVerticesX` can't coerce a multi-gigabyte allocation.
const MAX_VERTICES_PER_AXIS: u32 = 4096;

/// Reject implausible `WeightedTextureMaps` array counts. Shipped
/// content uses 1–3.
const MAX_WEIGHTED_TEXTURE_MAPS: u32 = 1024;

/// Deserialize a `Terrain` actor from export serial data.
///
/// `data` is the raw bytes from `pkg.read_export_data(export)`; `names`
/// is the package name table.
///
/// Every modelled section is bounds-checked against `data`, and the
/// count of bytes left over after `WeightMapTextures.Num` is reported in
/// [`Terrain::lighting_trailer_bytes`] rather than being silently
/// ignored — the caller can assert it against ground truth.
pub fn deserialize_terrain(data: &[u8], names: &[NameEntry]) -> Result<Terrain> {
    if data.len() <= ACTOR_HEADER_SIZE {
        return Err(ObjectError::InvalidData(format!(
            "Terrain export is {} bytes, too small for the {}-byte actor header",
            data.len(),
            ACTOR_HEADER_SIZE
        )));
    }

    let (props, mut pos) =
        cimmeria_upk::parse_tagged_properties_with_end(data, ACTOR_HEADER_SIZE, names);

    let num_patches_x = find_u32(&props, "NumPatchesX")?;
    let num_patches_y = find_u32(&props, "NumPatchesY")?;
    let num_vertices_x = find_u32(&props, "NumVerticesX")?;
    let num_vertices_y = find_u32(&props, "NumVerticesY")?;

    // UE3 invariant: the heightmap is one vertex wider than the patch
    // grid on each axis. If this doesn't hold, the property walk landed
    // somewhere wrong and every offset below is garbage — fail loudly
    // rather than decode nonsense heights.
    if num_vertices_x != num_patches_x + 1 || num_vertices_y != num_patches_y + 1 {
        return Err(ObjectError::InvalidData(format!(
            "Terrain grid mismatch: NumVertices {}x{} is not NumPatches {}x{} plus one",
            num_vertices_x, num_vertices_y, num_patches_x, num_patches_y
        )));
    }
    if num_vertices_x > MAX_VERTICES_PER_AXIS || num_vertices_y > MAX_VERTICES_PER_AXIS {
        return Err(ObjectError::InvalidData(format!(
            "Terrain grid {}x{} exceeds the {} per-axis cap",
            num_vertices_x, num_vertices_y, MAX_VERTICES_PER_AXIS
        )));
    }

    let expected_n = num_vertices_x as usize * num_vertices_y as usize;

    // --- binary trailer ---

    let heights_num = read_count(data, &mut pos, "Heights.Num")?;
    if heights_num as usize != expected_n {
        return Err(ObjectError::InvalidData(format!(
            "Heights.Num = {} but NumVerticesX * NumVerticesY = {}",
            heights_num, expected_n
        )));
    }
    ensure_bytes(data, pos, expected_n * 2, "Heights")?;
    let mut heights = Vec::with_capacity(expected_n);
    for k in 0..expected_n {
        heights.push(LittleEndian::read_u16(&data[pos + k * 2..]));
    }
    pos += expected_n * 2;

    let info_num = read_count(data, &mut pos, "InfoData.Num")?;
    if info_num as usize != expected_n {
        return Err(ObjectError::InvalidData(format!(
            "InfoData.Num = {} but Heights.Num = {}",
            info_num, expected_n
        )));
    }
    ensure_bytes(data, pos, expected_n, "InfoData")?;
    let info_data = data[pos..pos + expected_n].to_vec();
    pos += expected_n;

    // Binary copies of AlphaXSize/AlphaYSize. They must be consumed
    // regardless; we also cross-check them against the tagged-property
    // values when those are present, because a mismatch is the cheapest
    // available signal that the trailer walk has drifted.
    let alpha_x_size = read_count(data, &mut pos, "AlphaXSize")?;
    let alpha_y_size = read_count(data, &mut pos, "AlphaYSize")?;
    check_alpha(&props, "AlphaXSize", alpha_x_size)?;
    check_alpha(&props, "AlphaYSize", alpha_y_size)?;

    let weighted_texture_map_count = read_count(data, &mut pos, "WeightedTextureMaps.Num")?;
    if weighted_texture_map_count > MAX_WEIGHTED_TEXTURE_MAPS {
        return Err(ObjectError::InvalidData(format!(
            "WeightedTextureMaps.Num = {} exceeds the {} cap",
            weighted_texture_map_count, MAX_WEIGHTED_TEXTURE_MAPS
        )));
    }
    for i in 0..weighted_texture_map_count {
        let len = read_count(data, &mut pos, "WeightedTextureMaps[i].Num")? as usize;
        ensure_bytes(data, pos, len, "WeightedTextureMaps[i] payload")?;
        tracing::trace!(index = i, bytes = len, "skipping weighted texture map");
        pos += len;
    }

    let weight_map_texture_count = read_count(data, &mut pos, "WeightMapTextures.Num")?;

    // Everything from here on is lighting GUIDs + foliage proxy data.
    // `pos` can never exceed `data.len()` — every read above went
    // through `ensure_bytes` — so the subtraction is safe, but use a
    // checked form so a future edit can't turn it into a panic.
    let lighting_trailer_bytes = data.len().checked_sub(pos).ok_or_else(|| {
        ObjectError::InvalidData(format!(
            "Terrain trailer walk consumed {} of {} bytes",
            pos,
            data.len()
        ))
    })?;

    let terrain = Terrain {
        num_patches_x,
        num_patches_y,
        num_vertices_x,
        num_vertices_y,
        num_sections_x: find_u32_opt(&props, "NumSectionsX")?.unwrap_or(1),
        num_sections_y: find_u32_opt(&props, "NumSectionsY")?.unwrap_or(1),
        max_tesselation_level: find_u32_opt(&props, "MaxTesselationLevel")?.unwrap_or(1),
        location: find_vector(&props, "Location")?.unwrap_or([0.0; 3]),
        rotation: find_rotator(&props, "Rotation")?.unwrap_or([0; 3]),
        draw_scale: find_float(&props, "DrawScale")?.unwrap_or(1.0),
        draw_scale_3d: find_vector(&props, "DrawScale3D")?
            .unwrap_or(SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D),
        heights,
        info_data,
        alpha_x_size,
        alpha_y_size,
        weighted_texture_map_count,
        weight_map_texture_count,
        lighting_trailer_bytes,
    };

    tracing::debug!(
        patches = format!("{}x{}", terrain.num_patches_x, terrain.num_patches_y),
        sections = format!("{}x{}", terrain.num_sections_x, terrain.num_sections_y),
        location = ?terrain.location,
        draw_scale_3d = ?terrain.draw_scale_3d,
        lighting_trailer_bytes,
        "decoded Terrain"
    );

    Ok(terrain)
}

/// Read a `TArray` length / INT32 count and advance.
///
/// UE3 writes these as signed; a negative value means the trailer walk
/// has drifted (or the file is hostile), so reject it here rather than
/// letting a `as usize` cast wrap into a huge allocation.
fn read_count(data: &[u8], pos: &mut usize, field: &str) -> Result<u32> {
    ensure_bytes(data, *pos, 4, field)?;
    let raw = LittleEndian::read_i32(&data[*pos..]);
    *pos += 4;
    u32::try_from(raw).map_err(|_| {
        ObjectError::InvalidData(format!(
            "{} is negative ({}) at offset {}",
            field,
            raw,
            *pos - 4
        ))
    })
}

/// Cross-check a binary alpha-size copy against its tagged-property
/// twin. Absent property ⇒ nothing to check; a *malformed* one is an
/// error rather than a skipped check, because this cross-check is the
/// cheapest signal that the trailer walk has drifted and swallowing
/// it removes the very guard it exists to be.
fn check_alpha(props: &[TaggedProperty], name: &str, binary: u32) -> Result<()> {
    if let Some(from_props) = find_u32_opt(props, name)? {
        if from_props != binary {
            return Err(ObjectError::InvalidData(format!(
                "{} mismatch: tagged property says {}, binary trailer says {}",
                name, from_props, binary
            )));
        }
    }
    Ok(())
}

fn find_u32(props: &[TaggedProperty], name: &str) -> Result<u32> {
    let p = props
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| ObjectError::MissingProperty(name.to_string()))?;
    match &p.value {
        PropValue::Int(v) => u32::try_from(*v)
            .map_err(|_| ObjectError::InvalidData(format!("{} is negative ({})", name, v))),
        other => Err(ObjectError::InvalidData(format!(
            "{} is {:?}, expected IntProperty",
            name, other
        ))),
    }
}

/// [`find_u32`] for an optional property: absent is `Ok(None)`, but a
/// property that *is* present and holds the wrong type still fails.
///
/// The distinction matters because every optional terrain property has
/// a plausible default. Collapsing "absent" and "wrong type" into the
/// same `None` means a `Location` that decoded as something other than
/// a `Vector` silently places the terrain at the origin — a whole
/// chunk of ground in the wrong place, with nothing to say so.
fn find_u32_opt(props: &[TaggedProperty], name: &str) -> Result<Option<u32>> {
    match find_u32(props, name) {
        Ok(v) => Ok(Some(v)),
        Err(ObjectError::MissingProperty(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

fn find_vector(props: &[TaggedProperty], name: &str) -> Result<Option<[f32; 3]>> {
    match props.iter().find(|p| p.name == name) {
        None => Ok(None),
        Some(p) => match &p.value {
            PropValue::Vector { x, y, z } => Ok(Some([*x, *y, *z])),
            other => Err(ObjectError::InvalidData(format!(
                "{} is {:?}, expected a Vector StructProperty",
                name, other
            ))),
        },
    }
}

fn find_rotator(props: &[TaggedProperty], name: &str) -> Result<Option<[i32; 3]>> {
    match props.iter().find(|p| p.name == name) {
        None => Ok(None),
        Some(p) => match &p.value {
            PropValue::Rotator { pitch, yaw, roll } => Ok(Some([*pitch, *yaw, *roll])),
            other => Err(ObjectError::InvalidData(format!(
                "{} is {:?}, expected a Rotator StructProperty",
                name, other
            ))),
        },
    }
}

fn find_float(props: &[TaggedProperty], name: &str) -> Result<Option<f32>> {
    match props.iter().find(|p| p.name == name) {
        None => Ok(None),
        Some(p) => match &p.value {
            PropValue::Float(v) => Ok(Some(*v)),
            other => Err(ObjectError::InvalidData(format!(
                "{} is {:?}, expected FloatProperty",
                name, other
            ))),
        },
    }
}

/// Check that enough bytes remain for the next read.
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

#[cfg(test)]
mod tests;

//! Rebuild a world map's overview texture from the map's own tiles.
//!
//! A `<Map>_MapData.upk` holds one 256x256 DXT1 tile per 100 m chunk
//! (`thumb_WorldMap_<hi16><lo16>`) and one overview texture the client
//! actually draws. For Ihpet_Crater_Light the stock overview is a 1.99x zoom
//! of the map's top-left corner, so every icon and marker sits on the wrong
//! terrain. This reads the tiles out of the player's own package, stitches
//! and resamples them to the scale the map record describes, and writes the
//! overview texture again. The result is a pure function of the input file
//! and the [`WorldMapRebake`] parameters: integer arithmetic, our own DXT1
//! encoder and a pinned LZO, so a result hash pinned in a patch recipe holds
//! on every machine.
//!
//! Layout: `hi` is the north-south chunk row (the largest `hi` is the top
//! row), `lo` the east-west column (the smallest `lo` is the left column).
//! The map fills the top-left of the overview at `size / max(columns, rows)`
//! texels a chunk; the rest is `pad`, with the last picture column carried
//! into `carry` texels of it so no DXT1 block or bilinear tap mixes picture
//! with pad.

use std::path::Path;

use super::{dxt1, lzo_chunks, resample, texture2d};
use crate::error::{Result, UpkError};
use crate::patcher::PatchSession;

/// What to rebuild and how. Everything map-specific lives here, so a second
/// map needs a new recipe and no new code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMapRebake {
    /// Export name of the overview texture to replace.
    pub texture: String,
    /// Export name prefix of the tiles; the chunk follows as `<hi><lo>`,
    /// each four lower-case hex digits of the two's-complement `i16`.
    pub tile_prefix: String,
    /// Inclusive chunk column range (`lo`), west to east.
    pub lo: (i32, i32),
    /// Inclusive chunk row range (`hi`), south to north.
    pub hi: (i32, i32),
    /// Overview texture edge in texels.
    pub size: u32,
    /// RGB of the texels outside the map.
    pub pad: [u8; 3],
    /// Texels of the last picture column repeated into the pad.
    pub carry: u32,
}

/// Limits on what a recipe or a package may ask for. A recipe is signed and a
/// package is the player's own file, but a typo or a modified file must fail
/// one patch with an error, never allocate gigabytes and take the launcher
/// down with it.
pub const MAX_SIZE: u32 = 4096;
/// Chunk columns or rows.
pub const MAX_CHUNKS: i64 = 256;
/// Edge of one tile, texels.
pub const MAX_TILE_EDGE: usize = 1024;
/// Bytes of the stitched RGB mosaic.
pub const MAX_MOSAIC_BYTES: usize = 256 * 1024 * 1024;

/// Rebuild the overview texture of the package at `path` and return the new
/// package file.
pub fn rebake(path: &Path, p: &WorldMapRebake) -> Result<Vec<u8>> {
    // Checked before the package is opened or anything is allocated.
    chunk_counts(p)?;
    let mut session = PatchSession::open(path)?;
    let index = find_export(&session, &p.texture)?;
    let (mosaic, edge) = stitch(&session, p)?;
    let overview = overview(&mosaic, edge, p);

    let old = texture2d::parse(session.export_data(index)?, &session.package.names)?;
    let offset = session.replacement_offset(index);
    let mips = old
        .mips
        .iter()
        .map(|m| rebuild_mip(m, &overview, p.size as usize))
        .collect::<Result<Vec<_>>>()?;
    let data = texture2d::write(
        &texture2d::Texture2dData {
            head: old.head,
            mips,
        },
        offset,
    );
    session.replace_export_data(index, data)?;
    session.finish()
}

fn find_export(session: &PatchSession, name: &str) -> Result<usize> {
    session
        .package
        .exports
        .iter()
        .position(|e| e.object_name == name)
        .ok_or_else(|| UpkError::Parse(format!("the package has no export named {name}")))
}

/// Texels of a mip or tile edge from the file's `i32`, within `1..=max`.
fn edge(value: i32, max: usize, what: &str) -> Result<usize> {
    usize::try_from(value)
        .ok()
        .filter(|e| (1..=max).contains(e))
        .ok_or_else(|| UpkError::Parse(format!("{what} edge {value} is outside 1..={max}")))
}

/// Decode every tile and paste it where its chunk belongs.
/// Returns the mosaic and the edge of one tile in texels.
fn stitch(session: &PatchSession, p: &WorldMapRebake) -> Result<(Vec<u8>, usize)> {
    let (cols, rows) = chunk_counts(p)?;
    let mut tile_edge = 0usize;
    let mut mosaic = Vec::new();
    for hi in p.hi.0..=p.hi.1 {
        for lo in p.lo.0..=p.lo.1 {
            // Both are inside i16 (checked in `chunk_counts`), so the name
            // cannot alias another chunk's.
            let name = format!(
                "{}{:04x}{:04x}",
                p.tile_prefix, hi as i16 as u16, lo as i16 as u16
            );
            let index = find_export(session, &name)?;
            let tex = texture2d::parse(session.export_data(index)?, &session.package.names)?;
            let top = tex
                .mips
                .first()
                .ok_or_else(|| UpkError::Parse(format!("{name} has no mips")))?;
            let w = edge(top.width, MAX_TILE_EDGE, &name)?;
            let h = edge(top.height, MAX_TILE_EDGE, &name)?;
            if w != h || !w.is_multiple_of(4) || (tile_edge != 0 && w != tile_edge) {
                return Err(UpkError::Parse(format!(
                    "{name} is {w}x{h}, not a square multiple-of-4 tile like the rest"
                )));
            }
            if mosaic.is_empty() {
                tile_edge = w;
                let bytes = cols
                    .checked_mul(rows)
                    .and_then(|c| c.checked_mul(w * h * 3))
                    .filter(|&b| b <= MAX_MOSAIC_BYTES)
                    .ok_or_else(|| UpkError::Parse("the tile mosaic would be too large".into()))?;
                mosaic = vec![0u8; bytes];
            }
            let raw = if top.flags & texture2d::FLAG_LZO != 0 {
                lzo_chunks::unpack(&top.payload, w * h / 2)?
            } else {
                top.payload.clone()
            };
            let rgb =
                dxt1::decode(&raw, w, h).map_err(|e| UpkError::Parse(format!("{name}: {e}")))?;
            let (col, row) = ((lo - p.lo.0) as usize, (p.hi.1 - hi) as usize);
            for y in 0..h {
                let to = (((row * h + y) * cols * w) + col * w) * 3;
                mosaic[to..to + w * 3].copy_from_slice(&rgb[y * w * 3..(y + 1) * w * 3]);
            }
        }
    }
    Ok((mosaic, tile_edge))
}

/// Validate the parameters and return the chunk columns and rows.
fn chunk_counts(p: &WorldMapRebake) -> Result<(usize, usize)> {
    let bad = |what: &str| UpkError::Parse(format!("the world map parameters are invalid: {what}"));
    if !(8..=MAX_SIZE).contains(&p.size) || !p.size.is_multiple_of(4) {
        return Err(bad("size must be a multiple of 4 in 8..=4096"));
    }
    if p.carry > p.size {
        return Err(bad("carry is larger than size"));
    }
    let span = |(a, b): (i32, i32), what: &str| {
        let in_i16 = |v: i32| i16::try_from(v).is_ok();
        let n = i64::from(b) - i64::from(a) + 1;
        if !in_i16(a) || !in_i16(b) || !(1..=MAX_CHUNKS).contains(&n) {
            return Err(bad(&format!(
                "{what} must be an ordered range inside i16 of at most {MAX_CHUNKS} chunks"
            )));
        }
        Ok(n as usize)
    };
    Ok((span(p.lo, "lo")?, span(p.hi, "hi")?))
}

/// The overview picture: the mosaic scaled to the map's share of the texture,
/// top-left, over `pad`.
fn overview(mosaic: &[u8], edge: usize, p: &WorldMapRebake) -> Vec<u8> {
    let (cols, rows) = chunk_counts(p).expect("checked while stitching");
    let size = p.size as usize;
    let longest = cols.max(rows);
    // Ceiling, so the picture reaches the edge of the map's share.
    let (w, h) = (
        (size * cols).div_ceil(longest),
        (size * rows).div_ceil(longest),
    );
    let picture = resample::area(mosaic, cols * edge, rows * edge, w, h);
    let mut out = vec![0u8; size * size * 3];
    for px in out.as_chunks_mut::<3>().0 {
        *px = p.pad;
    }
    for y in 0..h {
        out[y * size * 3..(y * size + w) * 3].copy_from_slice(&picture[y * w * 3..(y + 1) * w * 3]);
        let last = &picture[(y * w + w - 1) * 3..(y * w + w) * 3];
        for k in 0..(p.carry as usize).min(size - w) {
            out[(y * size + w + k) * 3..(y * size + w + k + 1) * 3].copy_from_slice(last);
        }
    }
    out
}

/// One mip of the new texture, in the stock mip's format and size.
fn rebuild_mip(old: &texture2d::Mip, overview: &[u8], size: usize) -> Result<texture2d::Mip> {
    let w = edge(old.width, size, "overview mip")?;
    let h = edge(old.height, size, "overview mip")?;
    let image = if (w, h) == (size, size) {
        overview.to_vec()
    } else {
        resample::area(overview, size, size, w, h)
    };
    // DXT1 blocks are 4x4: the tail mips store a whole block.
    let (bw, bh) = (w.max(4), h.max(4));
    let image = if (bw, bh) == (w, h) {
        image
    } else {
        resample::area(&image, w, h, bw, bh)
    };
    let raw = dxt1::encode(&image, bw, bh);
    let payload = if old.flags & texture2d::FLAG_LZO != 0 {
        lzo_chunks::pack(&raw)?
    } else {
        raw.clone()
    };
    Ok(texture2d::Mip {
        flags: old.flags,
        elements: raw.len() as i32,
        payload,
        width: old.width,
        height: old.height,
    })
}

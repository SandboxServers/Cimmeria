//! The `.occ` file: a small uncompressed header, then one zlib stream per
//! layer. All integers little-endian.
//!
//! ```text
//! header
//!   0   [u8; 4]  magic "CMOC"
//!   4   u16      version (1)
//!   6   u16      layer count
//!   8   f32      y_base   (metres at quantum 0)
//!  12   f32      y_step   (metres per quantum)
//!  16   u64      source hash (FNV-1a 64 over the input triangles)
//!  24   u16      label length, then that many UTF-8 bytes
//! per layer, 44 bytes (at most one terrain layer, after the span layers)
//!       u8       kind (0 geometry spans, 1 terrain heightfield), 3 zero bytes
//!       f32      cell edge (metres; the lattice pitch for terrain)
//!       f32 f32  origin x, z (world corner of tile 0,0)
//!       u32 u32  tiles_x, tiles_z
//!       u32      stored tiles
//!       u32      spans (terrain: patches holding a triangle)
//!       u32      uncompressed payload bytes
//!       u32      compressed payload bytes
//!       u32      reserved (0)
//! payloads, in layer order: zlib of
//!   slots:  tiles_x * tiles_z bytes, row-major by z: 0 uncovered,
//!           1 covered and empty, 2 stored
//!   per stored tile (slot order): 256 span counts, one byte each
//!   spans (stored tiles in slot order, cells row-major by z inside a
//!           tile, spans by ascending lo), LEB128 varints:
//!           first span of a cell: zigzag(lo - previous cell's first lo),
//!                                 hi - lo
//!           later spans:          lo - previous lo, hi - lo
//!   rects:  one u16 per span, same order: the span's sub-cell rectangle,
//!           nibbles x0 x1 z0 z1 (high to low), inclusive, 16 per cell side
//! terrain payload: zlib of
//!   slots:  as above
//!   per stored tile: f32 base (world Y of height 0)
//!   per stored tile: 17 x 17 vertex heights, row-major by z, in 1 cm
//!           quanta above the base, zigzag varint deltas (first against 0)
//!   per stored tile: 256 patch flag bytes: bit 0 / bit 1 = first / second
//!           triangle present, bit 2 = split along the (x0,z1)-(x1,z0)
//!           diagonal
//! ```
//!
//! The delta coding is what makes terrain nearly free after zlib: the next
//! cell's ground is almost always within a quantum or two of this one's,
//! and nearly every terrain rectangle is the whole cell.

use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::grid::{
    fnv1a64, Layer, LayerKind, Occluder, Span, SubRect, TileHead, SLOT_EMPTY, SLOT_UNCOVERED,
    TILE_CELLS, VARIABLE,
};
use crate::heightfield::{Heightfield, ANTI, HAS_A, HAS_B, TILE_VERTS};

/// File magic.
pub const MAGIC: [u8; 4] = *b"CMOC";
/// Current format version.
pub const VERSION: u16 = 1;

/// Hostile-input guards. The largest shipped world (see the data README) is
/// far inside each.
const MAX_LAYERS: u16 = 4;
const MAX_TILES: u64 = 64 << 20;
const MAX_SPANS: u64 = 1 << 30;
const MAX_PAYLOAD: u64 = 1 << 31;

/// Why an `.occ` file could not be read.
#[derive(Debug, thiserror::Error)]
pub enum OccluderError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not an occluder file (bad magic)")]
    BadMagic,
    #[error("unsupported occluder version {0}")]
    Version(u16),
    #[error("malformed occluder file: {0}")]
    Malformed(&'static str),
}

struct Cursor<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], OccluderError> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&e| e <= self.b.len())
            .ok_or(OccluderError::Malformed("truncated"))?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, OccluderError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, OccluderError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().unwrap_or([0; 2]),
        ))
    }
    fn u32(&mut self) -> Result<u32, OccluderError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().unwrap_or([0; 4]),
        ))
    }
    fn u64(&mut self) -> Result<u64, OccluderError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().unwrap_or([0; 8]),
        ))
    }
    fn f32(&mut self) -> Result<f32, OccluderError> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn varint(&mut self) -> Result<u64, OccluderError> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let b = self.u8()?;
            v |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err(OccluderError::Malformed("varint overflow"))
    }
}

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

fn unzigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

/// A layer's spans, cell by cell: `(start, end)` into `layer.spans` per
/// cell of one stored tile.
fn tile_cells<'a>(layer: &'a Layer, head: &TileHead) -> impl Iterator<Item = (usize, usize)> + 'a {
    let head = *head;
    (0..TILE_CELLS).map(move |i| {
        let base = head.span_base as usize;
        if head.uniform != VARIABLE {
            let k = head.uniform as usize;
            (base + i * k, base + i * k + k)
        } else {
            let ends = &layer.cell_end[head.ends as usize..head.ends as usize + TILE_CELLS];
            let s = if i == 0 { 0 } else { ends[i - 1] as usize };
            (base + s, base + ends[i] as usize)
        }
    })
}

fn layer_payload(layer: &Layer) -> Vec<u8> {
    let mut out = Vec::with_capacity(layer.slots.len() + layer.spans.len() * 2);
    out.extend(layer.slots.iter().map(|&s| slot_byte(s)));
    let stored: Vec<&TileHead> = layer
        .slots
        .iter()
        .filter(|&&s| s != SLOT_UNCOVERED && s != SLOT_EMPTY)
        .map(|&s| &layer.heads[s as usize])
        .collect();
    for head in &stored {
        for (s, e) in tile_cells(layer, head) {
            out.push((e - s) as u8);
        }
    }
    let mut prev_first: i64 = 0;
    for head in &stored {
        for (s, e) in tile_cells(layer, head) {
            let mut prev_lo: Option<i64> = None;
            for sp in &layer.spans[s..e] {
                let (lo, hi) = (sp.lo as i64, sp.hi as i64);
                match prev_lo {
                    None => {
                        put_varint(&mut out, zigzag(lo - prev_first));
                        prev_first = lo;
                    }
                    Some(pl) => put_varint(&mut out, (lo - pl) as u64),
                }
                put_varint(&mut out, (hi - lo) as u64);
                prev_lo = Some(lo);
            }
        }
    }
    for head in &stored {
        for (s, e) in tile_cells(layer, head) {
            for sp in &layer.spans[s..e] {
                out.extend_from_slice(&sp.rect.to_le_bytes());
            }
        }
    }
    out
}

fn slot_byte(s: u32) -> u8 {
    match s {
        SLOT_UNCOVERED => 0,
        SLOT_EMPTY => 1,
        _ => 2,
    }
}

fn heightfield_payload(h: &Heightfield) -> Vec<u8> {
    let mut out: Vec<u8> = h.slots.iter().map(|&s| slot_byte(s)).collect();
    for b in &h.base {
        out.extend_from_slice(&b.to_le_bytes());
    }
    for tile in h.heights.as_chunks::<TILE_VERTS>().0 {
        let mut prev = 0i64;
        for &v in tile {
            put_varint(&mut out, zigzag(v as i64 - prev));
            prev = v as i64;
        }
    }
    out.extend_from_slice(&h.flags);
    out
}

fn zlib(raw: &[u8]) -> Vec<u8> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::best());
    // Writing into a Vec cannot fail.
    let _ = z.write_all(raw);
    z.finish().unwrap_or_default()
}

/// Encode an occluder to `.occ` bytes.
pub fn encode(occ: &Occluder) -> Vec<u8> {
    let mut out = Vec::new();
    let entries = occ.layers.len() + usize::from(occ.heightfield.is_some());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(entries as u16).to_le_bytes());
    out.extend_from_slice(&occ.y_base.to_le_bytes());
    out.extend_from_slice(&occ.y_step.to_le_bytes());
    out.extend_from_slice(&occ.source_hash.to_le_bytes());
    let label = occ.label.as_bytes();
    let label = &label[..label.len().min(u16::MAX as usize)];
    out.extend_from_slice(&(label.len() as u16).to_le_bytes());
    out.extend_from_slice(label);
    // (kind, cell, origin, tiles, stored, count, raw)
    let mut entries: Vec<(LayerKind, f32, [f32; 2], [u32; 2], u32, u32, Vec<u8>)> = occ
        .layers
        .iter()
        .map(|l| {
            (
                l.kind,
                l.cell,
                l.origin,
                [l.tiles_x, l.tiles_z],
                l.heads.len() as u32,
                l.spans.len() as u32,
                layer_payload(l),
            )
        })
        .collect();
    if let Some(h) = &occ.heightfield {
        entries.push((
            LayerKind::Terrain,
            h.pitch,
            h.origin,
            [h.tiles_x, h.tiles_z],
            h.base.len() as u32,
            h.patch_count() as u32,
            heightfield_payload(h),
        ));
    }
    let packed: Vec<Vec<u8>> = entries.iter().map(|e| zlib(&e.6)).collect();
    for (e, z) in entries.iter().zip(&packed) {
        out.push(e.0 as u8);
        out.extend_from_slice(&[0, 0, 0]);
        out.extend_from_slice(&e.1.to_le_bytes());
        out.extend_from_slice(&e.2[0].to_le_bytes());
        out.extend_from_slice(&e.2[1].to_le_bytes());
        out.extend_from_slice(&e.3[0].to_le_bytes());
        out.extend_from_slice(&e.3[1].to_le_bytes());
        out.extend_from_slice(&e.4.to_le_bytes());
        out.extend_from_slice(&e.5.to_le_bytes());
        out.extend_from_slice(&(e.6.len() as u32).to_le_bytes());
        out.extend_from_slice(&(z.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    for z in &packed {
        out.extend_from_slice(z);
    }
    out
}

struct LayerHeader {
    kind: LayerKind,
    cell: f32,
    origin: [f32; 2],
    tiles_x: u32,
    tiles_z: u32,
    stored: u32,
    spans: u32,
    raw_len: u32,
    packed_len: u32,
}

/// Decode `.occ` bytes.
pub fn decode(bytes: &[u8]) -> Result<Occluder, OccluderError> {
    let mut c = Cursor { b: bytes, at: 0 };
    if c.take(4)? != MAGIC {
        return Err(OccluderError::BadMagic);
    }
    let version = c.u16()?;
    if version != VERSION {
        return Err(OccluderError::Version(version));
    }
    let n_layers = c.u16()?;
    if n_layers == 0 || n_layers > MAX_LAYERS {
        return Err(OccluderError::Malformed("layer count"));
    }
    let y_base = c.f32()?;
    let y_step = c.f32()?;
    if !(y_base.is_finite() && y_step.is_finite() && y_step > 0.0) {
        return Err(OccluderError::Malformed("y quantisation"));
    }
    let source_hash = c.u64()?;
    let label_len = c.u16()? as usize;
    let label = String::from_utf8(c.take(label_len)?.to_vec())
        .map_err(|_| OccluderError::Malformed("label is not UTF-8"))?;
    let mut headers = Vec::with_capacity(n_layers as usize);
    for _ in 0..n_layers {
        let kind =
            LayerKind::from_u8(c.u8()?).ok_or(OccluderError::Malformed("unknown layer kind"))?;
        c.take(3)?;
        let h = LayerHeader {
            kind,
            cell: c.f32()?,
            origin: [c.f32()?, c.f32()?],
            tiles_x: c.u32()?,
            tiles_z: c.u32()?,
            stored: c.u32()?,
            spans: c.u32()?,
            raw_len: c.u32()?,
            packed_len: c.u32()?,
        };
        c.u32()?;
        let tiles = h.tiles_x as u64 * h.tiles_z as u64;
        if !(h.cell.is_finite() && h.cell > 0.0)
            || !(h.origin[0].is_finite() && h.origin[1].is_finite())
            || tiles == 0
            || tiles > MAX_TILES
            || h.stored as u64 > tiles
            || h.spans as u64 > MAX_SPANS
            || h.raw_len as u64 > MAX_PAYLOAD
        {
            return Err(OccluderError::Malformed("layer header out of range"));
        }
        headers.push(h);
    }
    let mut layers = Vec::with_capacity(headers.len());
    let mut heightfield = None;
    for h in headers {
        let packed = c.take(h.packed_len as usize)?;
        let mut raw = Vec::with_capacity(h.raw_len as usize);
        ZlibDecoder::new(packed)
            .take(h.raw_len as u64 + 1)
            .read_to_end(&mut raw)?;
        if raw.len() != h.raw_len as usize {
            return Err(OccluderError::Malformed("payload length"));
        }
        match h.kind {
            LayerKind::Geometry if heightfield.is_none() => layers.push(decode_layer(&h, &raw)?),
            LayerKind::Terrain if heightfield.is_none() => {
                heightfield = Some(decode_heightfield(&h, &raw)?)
            }
            _ => return Err(OccluderError::Malformed("layer order")),
        }
    }
    if c.at != bytes.len() {
        return Err(OccluderError::Malformed("trailing bytes"));
    }
    let mut occ = Occluder {
        y_base,
        y_step,
        source_hash,
        label,
        layers,
        heightfield,
        content_hash: 0,
        short_hash: String::new(),
    };
    occ.set_content_hash(fnv1a64(bytes));
    Ok(occ)
}

fn decode_slots(c: &mut Cursor<'_>, n: usize) -> Result<(Vec<u32>, u32), OccluderError> {
    let mut slots = Vec::with_capacity(n);
    let mut stored = 0u32;
    for &b in c.take(n)? {
        slots.push(match b {
            0 => SLOT_UNCOVERED,
            1 => SLOT_EMPTY,
            2 => {
                stored += 1;
                stored - 1
            }
            _ => return Err(OccluderError::Malformed("slot byte")),
        });
    }
    Ok((slots, stored))
}

fn decode_heightfield(h: &LayerHeader, raw: &[u8]) -> Result<Heightfield, OccluderError> {
    let mut c = Cursor { b: raw, at: 0 };
    let (slots, stored) = decode_slots(&mut c, (h.tiles_x as usize) * (h.tiles_z as usize))?;
    if stored != h.stored {
        return Err(OccluderError::Malformed("stored tile count"));
    }
    let mut base = Vec::with_capacity(stored as usize);
    for _ in 0..stored {
        let b = c.f32()?;
        if !b.is_finite() {
            return Err(OccluderError::Malformed("terrain base"));
        }
        base.push(b);
    }
    let mut heights = Vec::with_capacity(stored as usize * TILE_VERTS);
    for _ in 0..stored {
        let mut prev = 0i64;
        for _ in 0..TILE_VERTS {
            let v = prev + unzigzag(c.varint()?);
            if !(0..=u16::MAX as i64).contains(&v) {
                return Err(OccluderError::Malformed("terrain height"));
            }
            heights.push(v as u16);
            prev = v;
        }
    }
    let flags = c.take(stored as usize * TILE_CELLS)?.to_vec();
    if flags.iter().any(|&f| f & !(HAS_A | HAS_B | ANTI) != 0) {
        return Err(OccluderError::Malformed("terrain flags"));
    }
    if c.at != raw.len() {
        return Err(OccluderError::Malformed("payload trailing bytes"));
    }
    let hf = Heightfield {
        pitch: h.cell,
        origin: h.origin,
        tiles_x: h.tiles_x,
        tiles_z: h.tiles_z,
        slots,
        base,
        heights,
        flags,
    };
    if hf.patch_count() as u32 != h.spans {
        return Err(OccluderError::Malformed("terrain patch count"));
    }
    Ok(hf)
}

fn decode_layer(h: &LayerHeader, raw: &[u8]) -> Result<Layer, OccluderError> {
    let n_tiles = (h.tiles_x as usize) * (h.tiles_z as usize);
    let mut c = Cursor { b: raw, at: 0 };
    let (slots, stored) = decode_slots(&mut c, n_tiles)?;
    if stored != h.stored {
        return Err(OccluderError::Malformed("stored tile count"));
    }
    let counts = c.take(stored as usize * TILE_CELLS)?;
    let mut heads = Vec::with_capacity(stored as usize);
    let mut cell_end = Vec::new();
    let mut total = 0u64;
    for tile in counts.as_chunks::<TILE_CELLS>().0 {
        let first = tile[0];
        let uniform = first != VARIABLE && tile.iter().all(|&k| k == first);
        let mut head = TileHead {
            span_base: total as u32,
            uniform: if uniform { first } else { VARIABLE },
            ends: 0,
        };
        let mut end = 0u32;
        if !uniform {
            head.ends = cell_end.len() as u32;
        }
        for &k in tile {
            if k == VARIABLE {
                return Err(OccluderError::Malformed("span count byte"));
            }
            end += k as u32;
            if !uniform {
                cell_end.push(end as u16);
            }
        }
        total += end as u64;
        heads.push(head);
    }
    if total != h.spans as u64 {
        return Err(OccluderError::Malformed("span count"));
    }
    let mut spans = Vec::with_capacity(total as usize);
    let mut prev_first: i64 = 0;
    for tile in counts.as_chunks::<TILE_CELLS>().0 {
        for &k in tile {
            let mut prev_lo: Option<i64> = None;
            for _ in 0..k {
                let lo = match prev_lo {
                    None => {
                        let lo = prev_first + unzigzag(c.varint()?);
                        prev_first = lo;
                        lo
                    }
                    Some(pl) => pl + c.varint()? as i64,
                };
                let hi = lo + c.varint()? as i64;
                if lo < 0 || hi > u16::MAX as i64 {
                    return Err(OccluderError::Malformed("span out of range"));
                }
                spans.push(Span {
                    lo: lo as u16,
                    hi: hi as u16,
                    rect: SubRect::FULL.pack(),
                });
                prev_lo = Some(lo);
            }
        }
    }
    for sp in &mut spans {
        let rect = c.u16()?;
        if !SubRect::unpack(rect).is_valid() {
            return Err(OccluderError::Malformed("span rectangle"));
        }
        sp.rect = rect;
    }
    if c.at != raw.len() {
        return Err(OccluderError::Malformed("payload trailing bytes"));
    }
    Ok(Layer {
        kind: h.kind,
        cell: h.cell,
        origin: h.origin,
        tiles_x: h.tiles_x,
        tiles_z: h.tiles_z,
        slots,
        heads,
        cell_end,
        spans,
    })
}

impl Occluder {
    /// Read an `.occ` file.
    pub fn load(path: &std::path::Path) -> Result<Self, OccluderError> {
        decode(&std::fs::read(path)?)
    }

    /// Write this occluder as an `.occ` file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), OccluderError> {
        std::fs::write(path, encode(self))?;
        Ok(())
    }
}

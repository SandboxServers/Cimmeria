//! The terrain layer: an exact heightfield on the terrain's own lattice.
//!
//! SGW terrain is a grid of 1 m patches, two triangles each, in world-axis
//! alignment (every shipped map; see `navmesh-extractor/src/terrain.rs`).
//! A span column over a sloped patch has to hold the patch's whole min-max
//! height range, which blinds a ray skimming a hillside (Castle: two thirds
//! of the span grid's false blocks were terrain). Here the layer stores the
//! lattice vertex heights and, per patch, which halves exist (holes) and
//! which diagonal splits it, and a query tests the segment against the
//! actual triangles.
//!
//! The builder only accepts a triangle that is exactly one half of a
//! lattice patch and agrees with what the patch already holds; anything
//! else (a rotated or rescaled terrain, two overlapping terrains that
//! disagree) is handed back to the caller for the span layer, so nothing is
//! lost, only stored less tightly.

use std::collections::{HashMap, HashSet};

use crate::grid::{SLOT_EMPTY, SLOT_UNCOVERED, TILE, TILE_CELLS, TILE_MASK, TILE_SHIFT};
use crate::raster::Triangle;

/// Vertices per stored tile side.
pub(crate) const VERTS: usize = TILE + 1;
pub(crate) const TILE_VERTS: usize = VERTS * VERTS;
/// Height quantum, metres.
pub(crate) const H_STEP: f32 = 0.01;
/// How far off the lattice a vertex may sit and still count as on it.
const LATTICE_EPS: f32 = 2e-3;

/// Patch flags.
pub(crate) const HAS_A: u8 = 1;
pub(crate) const HAS_B: u8 = 2;
/// The patch splits along the `(x0, z1)`-`(x1, z0)` diagonal; otherwise
/// along `(x0, z0)`-`(x1, z1)`.
pub(crate) const ANTI: u8 = 4;

/// The runtime heightfield.
#[derive(Debug, Clone, PartialEq)]
pub struct Heightfield {
    pub(crate) pitch: f32,
    pub(crate) origin: [f32; 2],
    pub(crate) tiles_x: u32,
    pub(crate) tiles_z: u32,
    pub(crate) slots: Vec<u32>,
    /// Per stored tile: world Y of height quantum 0.
    pub(crate) base: Vec<f32>,
    /// Per stored tile, [`TILE_VERTS`] heights, row-major by z.
    pub(crate) heights: Vec<u16>,
    /// Per stored tile, [`TILE_CELLS`] patch flags, row-major by z.
    pub(crate) flags: Vec<u8>,
}

impl Heightfield {
    /// Patch edge, metres.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Grid dimensions in patches, `(x, z)`.
    pub fn dims(&self) -> (u32, u32) {
        (self.tiles_x * TILE as u32, self.tiles_z * TILE as u32)
    }

    /// Tiles holding terrain.
    pub fn stored_tiles(&self) -> usize {
        self.base.len()
    }

    /// Tiles inside the coverage mask.
    pub fn covered_tiles(&self) -> usize {
        self.slots.iter().filter(|&&s| s != SLOT_UNCOVERED).count()
    }

    /// Patches with at least one triangle.
    pub fn patch_count(&self) -> usize {
        self.flags
            .iter()
            .filter(|&&f| f & (HAS_A | HAS_B) != 0)
            .count()
    }

    /// Bytes on the heap.
    pub fn ram_bytes(&self) -> usize {
        self.slots.len() * 4 + self.base.len() * 4 + self.heights.len() * 2 + self.flags.len()
    }

    fn slot_of(&self, px: i64, pz: i64) -> Option<u32> {
        if px < 0 || pz < 0 {
            return None;
        }
        let (tx, tz) = (px >> TILE_SHIFT, pz >> TILE_SHIFT);
        if tx >= self.tiles_x as i64 || tz >= self.tiles_z as i64 {
            return None;
        }
        Some(self.slots[(tz * self.tiles_x as i64 + tx) as usize])
    }

    /// Whether world `(x, z)` lies in a covered tile.
    pub fn covers(&self, x: f32, z: f32) -> bool {
        let (px, pz) = self.patch_of(x, z);
        matches!(self.slot_of(px, pz), Some(s) if s != SLOT_UNCOVERED)
    }

    pub(crate) fn patch_of(&self, x: f32, z: f32) -> (i64, i64) {
        (
            ((x - self.origin[0]) / self.pitch).floor() as i64,
            ((z - self.origin[1]) / self.pitch).floor() as i64,
        )
    }

    /// The triangles of patch `(px, pz)`, world metres.
    pub(crate) fn patch_triangles(&self, px: i64, pz: i64) -> ([Triangle; 2], u8) {
        let empty = ([[[0.0; 3]; 3]; 2], 0);
        let slot = match self.slot_of(px, pz) {
            None | Some(SLOT_UNCOVERED) | Some(SLOT_EMPTY) => return empty,
            Some(s) => s as usize,
        };
        let (lx, lz) = ((px & TILE_MASK) as usize, (pz & TILE_MASK) as usize);
        let f = self.flags[slot * TILE_CELLS + lz * TILE + lx];
        if f & (HAS_A | HAS_B) == 0 {
            return empty;
        }
        let hs = &self.heights[slot * TILE_VERTS..(slot + 1) * TILE_VERTS];
        let base = self.base[slot];
        let x0 = self.origin[0] + px as f32 * self.pitch;
        let z0 = self.origin[1] + pz as f32 * self.pitch;
        let v = |dx: usize, dz: usize| {
            [
                x0 + dx as f32 * self.pitch,
                base + hs[(lz + dz) * VERTS + lx + dx] as f32 * H_STEP,
                z0 + dz as f32 * self.pitch,
            ]
        };
        let (c00, c10, c01, c11) = (v(0, 0), v(1, 0), v(0, 1), v(1, 1));
        let tris = if f & ANTI != 0 {
            [[c10, c01, c00], [c10, c01, c11]]
        } else {
            [[c00, c11, c10], [c00, c11, c01]]
        };
        (tris, f)
    }

    /// Where the segment `a -> b`, restricted to parameters `[t0, t1]`, first
    /// crosses the terrain.
    pub(crate) fn first_hit(&self, a: [f32; 3], b: [f32; 3], t0: f32, t1: f32) -> Option<[f32; 3]> {
        let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let at = |t: f32| [a[0] + d[0] * t, a[1] + d[1] * t, a[2] + d[2] * t];
        let (p0, p1) = (at(t0), at(t1));
        let mut hit = None;
        walk(self.origin, self.pitch, p0, p1, |px, pz| {
            let (tris, f) = self.patch_triangles(px, pz);
            for (i, t) in tris.iter().enumerate() {
                let bit = if i == 0 { HAS_A } else { HAS_B };
                if f & bit != 0 && crate::raster::segment_hits_triangle(p0, p1, t) {
                    hit = Some(p0);
                    return true;
                }
            }
            false
        });
        hit
    }
}

/// Visit every lattice cell the XZ projection of `p0 -> p1` crosses, in
/// order, until `visit` returns true.
pub(crate) fn walk(
    origin: [f32; 2],
    pitch: f32,
    p0: [f32; 3],
    p1: [f32; 3],
    mut visit: impl FnMut(i64, i64) -> bool,
) {
    let cell = |x: f32, o: f32| ((x - o) / pitch).floor() as i64;
    let (mut cx, mut cz) = (cell(p0[0], origin[0]), cell(p0[2], origin[1]));
    let (ex, ez) = (cell(p1[0], origin[0]), cell(p1[2], origin[1]));
    let (dx, dz) = (p1[0] - p0[0], p1[2] - p0[2]);
    let axis = |d: f32, p: f32, o: f32, c: i64| -> (i64, f32, f32) {
        if d > 0.0 {
            (1, pitch / d, (o + (c + 1) as f32 * pitch - p) / d)
        } else if d < 0.0 {
            (-1, pitch / -d, (o + c as f32 * pitch - p) / d)
        } else {
            (0, f32::INFINITY, f32::INFINITY)
        }
    };
    let (sx, tdx, mut tmx) = axis(dx, p0[0], origin[0], cx);
    let (sz, tdz, mut tmz) = axis(dz, p0[2], origin[1], cz);
    let steps = (ex - cx).abs() + (ez - cz).abs() + 2;
    for _ in 0..=steps {
        if visit(cx, cz) {
            return;
        }
        if tmx.min(tmz) > 1.0 {
            return;
        }
        if tmx < tmz {
            cx += sx;
            tmx += tdx;
        } else {
            cz += sz;
            tmz += tdz;
        }
    }
}

#[derive(Clone)]
struct TileAcc {
    heights: Vec<f32>,
    flags: Vec<u8>,
}

impl Default for TileAcc {
    fn default() -> Self {
        Self {
            heights: vec![f32::NAN; TILE_VERTS],
            flags: vec![0; TILE_CELLS],
        }
    }
}

/// Accumulates lattice-aligned terrain triangles.
pub(crate) struct HeightfieldAcc {
    pitch: f32,
    tiles: HashMap<(i64, i64), TileAcc>,
}

impl HeightfieldAcc {
    pub fn new(pitch: f32) -> Self {
        Self {
            pitch,
            tiles: HashMap::new(),
        }
    }

    /// Take `tri` if it is exactly one half of a lattice patch consistent
    /// with what the patch already holds. `false` hands it back.
    pub fn add(&mut self, tri: &Triangle) -> bool {
        let p = self.pitch;
        let mut ix = [0i64; 3];
        let mut iz = [0i64; 3];
        for (k, v) in tri.iter().enumerate() {
            let (fx, fz) = (v[0] / p, v[2] / p);
            let (rx, rz) = (fx.round(), fz.round());
            if ((fx - rx) * p).abs() > LATTICE_EPS || ((fz - rz) * p).abs() > LATTICE_EPS {
                return false;
            }
            ix[k] = rx as i64;
            iz[k] = rz as i64;
        }
        let (px, pz) = (
            *ix.iter().min().unwrap_or(&0),
            *iz.iter().min().unwrap_or(&0),
        );
        if ix.iter().any(|&x| x > px + 1) || iz.iter().any(|&z| z > pz + 1) {
            return false;
        }
        // Which of the four corners the triangle uses.
        let mut used = [false; 4]; // c00, c10, c01, c11
        for k in 0..3 {
            let c = ((ix[k] - px) + 2 * (iz[k] - pz)) as usize;
            if used[c] {
                return false; // degenerate: a corner twice
            }
            used[c] = true;
        }
        let missing = used.iter().position(|u| !u).unwrap_or(0);
        // Missing c00 or c11: the diagonal is c10-c01 (anti).
        let (anti, half) = match missing {
            3 => (true, HAS_A),
            0 => (true, HAS_B),
            2 => (false, HAS_A),
            _ => (false, HAS_B),
        };
        let tile = (px >> TILE_SHIFT, pz >> TILE_SHIFT);
        let (lx, lz) = ((px & TILE_MASK) as usize, (pz & TILE_MASK) as usize);
        let acc = self.tiles.entry(tile).or_default();
        let f = acc.flags[lz * TILE + lx];
        if f & (HAS_A | HAS_B) != 0 && ((f & ANTI != 0) != anti || f & half != 0) {
            return false;
        }
        for (k, v) in tri.iter().enumerate() {
            let vi = (lz + (iz[k] - pz) as usize) * VERTS + lx + (ix[k] - px) as usize;
            let h = acc.heights[vi];
            if !h.is_nan() && (h - v[1]).abs() > H_STEP {
                return false;
            }
        }
        for (k, v) in tri.iter().enumerate() {
            let vi = (lz + (iz[k] - pz) as usize) * VERTS + lx + (ix[k] - px) as usize;
            acc.heights[vi] = v[1];
        }
        acc.flags[lz * TILE + lx] = f | half | if anti { ANTI } else { 0 };
        true
    }

    /// Tile coordinates holding terrain.
    pub fn tile_keys(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        self.tiles.keys().copied()
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Lay the covered tiles out. `None` when nothing is covered or stored.
    pub fn finish(self, covered: &HashSet<(i64, i64)>) -> Option<Heightfield> {
        let (mut tx0, mut tx1, mut tz0, mut tz1) = (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
        for &(tx, tz) in covered {
            tx0 = tx0.min(tx);
            tx1 = tx1.max(tx);
            tz0 = tz0.min(tz);
            tz1 = tz1.max(tz);
        }
        if tx0 > tx1 {
            return None;
        }
        let tiles_x = (tx1 - tx0 + 1) as u32;
        let tiles_z = (tz1 - tz0 + 1) as u32;
        let mut hf = Heightfield {
            pitch: self.pitch,
            origin: [
                (tx0 * TILE as i64) as f32 * self.pitch,
                (tz0 * TILE as i64) as f32 * self.pitch,
            ],
            tiles_x,
            tiles_z,
            slots: vec![SLOT_UNCOVERED; tiles_x as usize * tiles_z as usize],
            base: Vec::new(),
            heights: Vec::new(),
            flags: Vec::new(),
        };
        let mut tiles = self.tiles;
        for tz in tz0..=tz1 {
            for tx in tx0..=tx1 {
                if !covered.contains(&(tx, tz)) {
                    continue;
                }
                let slot = ((tz - tz0) * tiles_x as i64 + (tx - tx0)) as usize;
                let Some(acc) = tiles.remove(&(tx, tz)) else {
                    hf.slots[slot] = SLOT_EMPTY;
                    continue;
                };
                let lo = acc
                    .heights
                    .iter()
                    .filter(|h| !h.is_nan())
                    .fold(f32::INFINITY, |m, &h| m.min(h));
                let base = (lo / H_STEP).floor() * H_STEP;
                hf.slots[slot] = hf.base.len() as u32;
                hf.base.push(base);
                hf.heights.extend(acc.heights.iter().map(|&h| {
                    if h.is_nan() {
                        0
                    } else {
                        ((h - base) / H_STEP).round().clamp(0.0, u16::MAX as f32) as u16
                    }
                }));
                hf.flags.extend_from_slice(&acc.flags);
            }
        }
        (!hf.base.is_empty()).then_some(hf)
    }
}

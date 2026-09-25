//! Building an [`Occluder`] from collision triangles.
//!
//! Feed every triangle through [`OccluderBuilder::add_triangle`] (in any
//! order, chunk by chunk: nothing is kept per triangle), then call
//! [`OccluderBuilder::finish`]. Each triangle is rasterised into every cell
//! its XZ footprint touches, as the Y range it occupies over that cell
//! ([`crate::raster::clip_y_range`]); overlapping ranges in a cell merge.
//!
//! **Coverage.** With a margin set, only tiles within `margin` of a
//! floor-like triangle (a surface within 45 degrees of level, either
//! winding) are kept: where nothing can stand, nothing needs to see. A thick
//! wall keeps its outer `margin` on both sides, which is all a ray between
//! two standing points ever reaches.

use std::collections::{HashMap, HashSet};

use crate::format;
use crate::grid::{
    Layer, LayerKind, Occluder, Span, SubRect, TileHead, SLOT_EMPTY, SLOT_UNCOVERED, SUB, TILE,
    TILE_CELLS, TILE_SHIFT, VARIABLE,
};
use crate::heightfield::HeightfieldAcc;
use crate::raster::{clip_to_cell, is_floor_like, Triangle};

/// Where a triangle came from. Terrain can go to its own, coarser layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// StaticMesh or BSP.
    Geometry,
    /// A `Terrain` heightfield patch.
    Terrain,
}

/// Build knobs. [`BuildParams::default`] is the shipped configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildParams {
    /// Geometry-layer cell edge, metres.
    pub cell: f32,
    /// Terrain lattice pitch, metres: terrain triangles that are exact
    /// halves of this lattice's patches go to the heightfield
    /// ([`crate::heightfield`]); every other terrain triangle goes to the
    /// geometry layer. `None` puts all terrain in the geometry layer. SGW
    /// terrain patches are 1 m.
    pub terrain_pitch: Option<f32>,
    /// Y quantum, metres.
    pub y_step: f32,
    /// Spans in one cell closer than this merge into one, metres. A gap that
    /// small is a seam between two meshes, not a window.
    pub merge_gap: f32,
    /// Keep only tiles within this distance of a floor-like surface;
    /// `None` keeps every tile that holds geometry.
    pub margin: Option<f32>,
    /// Steepest surface, degrees from level, that counts as floor-like for
    /// the coverage mask.
    pub max_floor_slope_deg: f32,
}

impl Default for BuildParams {
    fn default() -> Self {
        Self {
            cell: 0.5,
            terrain_pitch: Some(1.0),
            y_step: 0.1,
            merge_gap: 0.1,
            margin: Some(2.0),
            max_floor_slope_deg: 45.0,
        }
    }
}

/// Why a build could not produce an occluder.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum BuildError {
    #[error("no triangles were added")]
    Empty,
    #[error("invalid parameter: {0}")]
    BadParams(&'static str),
    #[error("the geometry spans {0} Y quanta, more than a u16 holds; raise y_step")]
    YRangeTooLarge(i64),
}

/// Quantised heights are packed with this bias so a signed quantum fits an
/// unsigned 20-bit field (+-52 km at 0.1 m).
const Q_BIAS: i64 = 1 << 19;
const Q_MAX: i64 = Q_BIAS - 1;
/// The most spans one cell may hold (the per-cell count is a byte on disk).
const MAX_SPANS_PER_CELL: usize = 254;
/// Two overlapping spans merge when the union box adds at most a
/// `1 / MERGE_PHANTOM_SHARE` share of its own volume as solid space that
/// neither had. Coplanar wall triangles and stacked pieces of one wall
/// merge; a floor under the whole cell and a wall along its edge do not.
const MERGE_PHANTOM_SHARE: u64 = 50;

/// One accumulated span: Y quanta (unbiased) and its sub-cell rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rec {
    cell: u64,
    lo: i64,
    hi: i64,
    rect: SubRect,
}

impl Rec {
    /// `cell << 56 | (lo + bias) << 36 | (hi + bias) << 16 | rect`, so a
    /// sort orders by cell, then lo.
    fn pack(self) -> u64 {
        (self.cell << 56)
            | (((self.lo + Q_BIAS) as u64) << 36)
            | (((self.hi + Q_BIAS) as u64) << 16)
            | self.rect.pack() as u64
    }

    fn unpack(r: u64) -> Self {
        Self {
            cell: r >> 56,
            lo: ((r >> 36) & 0xF_FFFF) as i64 - Q_BIAS,
            hi: ((r >> 16) & 0xF_FFFF) as i64 - Q_BIAS,
            rect: SubRect::unpack((r & 0xFFFF) as u16),
        }
    }

    fn volume(&self) -> u64 {
        self.rect.area() as u64 * (self.hi - self.lo + 1) as u64
    }

    /// `self` grown to take in `o`, when that stays within
    /// [`MERGE_PHANTOM_SHARE`]; `None` otherwise, or when the two do not
    /// meet in Y (within `gap`).
    fn merged(&self, o: &Self, gap: i64) -> Option<Self> {
        if o.lo > self.hi + gap || self.lo > o.hi + gap {
            return None;
        }
        let u = Self {
            cell: self.cell,
            lo: self.lo.min(o.lo),
            hi: self.hi.max(o.hi),
            rect: self.rect.union(o.rect),
        };
        let phantom = u.volume().saturating_sub(self.volume() + o.volume());
        (phantom * MERGE_PHANTOM_SHARE <= u.volume()).then_some(u)
    }
}

#[derive(Default)]
struct TileAcc {
    /// Packed [`Rec`]s.
    recs: Vec<u64>,
    /// Length after the last compaction.
    clean: usize,
}

impl TileAcc {
    fn push(&mut self, rec: Rec, gap: i64) {
        self.recs.push(rec.pack());
        if self.recs.len() > 2 * self.clean + 1024 {
            self.compact(gap);
        }
    }

    /// Sort and merge the records in place.
    fn compact(&mut self, gap: i64) {
        self.recs.sort_unstable();
        let mut out: Vec<u64> = Vec::with_capacity(self.recs.len() / 2 + 1);
        let mut cell: Vec<Rec> = Vec::new();
        for &packed in &self.recs {
            let r = Rec::unpack(packed);
            if cell.first().is_some_and(|c| c.cell != r.cell) {
                flush_cell(&mut cell, &mut out);
            }
            merge_into(&mut cell, r, gap);
        }
        flush_cell(&mut cell, &mut out);
        self.clean = out.len();
        self.recs = out;
    }
}

fn flush_cell(cell: &mut Vec<Rec>, out: &mut Vec<u64>) {
    cell.sort_unstable_by_key(|r| (r.lo, r.hi, r.rect.pack()));
    out.extend(cell.drain(..).map(Rec::pack));
}

/// Add `r` to one cell's span list, merging it into a span it may join,
/// and then whatever the grown span can join in turn.
fn merge_into(cell: &mut Vec<Rec>, r: Rec, gap: i64) {
    let mut cur = r;
    loop {
        let found = cell
            .iter()
            .enumerate()
            .find_map(|(i, s)| s.merged(&cur, gap).map(|m| (i, m)));
        let Some((i, m)) = found else {
            cell.push(cur);
            return;
        };
        cell.swap_remove(i);
        cur = m;
    }
}

struct LayerAcc {
    kind: LayerKind,
    cell: f32,
    tiles: HashMap<(i64, i64), TileAcc>,
}

impl LayerAcc {
    fn new(kind: LayerKind, cell: f32) -> Self {
        Self {
            kind,
            cell,
            tiles: HashMap::new(),
        }
    }

    fn rasterise(&mut self, tri: &Triangle, y_step: f32, gap: i64) {
        let (mut x0, mut x1, mut z0, mut z1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for v in tri {
            x0 = x0.min(v[0]);
            x1 = x1.max(v[0]);
            z0 = z0.min(v[2]);
            z1 = z1.max(v[2]);
        }
        let c = self.cell;
        let sub = c / SUB as f32;
        let (cx0, cx1) = ((x0 / c).floor() as i64, (x1 / c).floor() as i64);
        let (cz0, cz1) = ((z0 / c).floor() as i64, (z1 / c).floor() as i64);
        // Sub-cell index of a coordinate inside the cell starting at `base`,
        // clamped; min and max both round down, so the rectangle (which
        // runs to the far edge of its last sub-cell) rounds outwards.
        let idx =
            |v: f32, base: f32| ((v - base) / sub).floor().clamp(0.0, (SUB - 1) as f32) as u16;
        for cz in cz0..=cz1 {
            let (zl, zh) = (cz as f32 * c, (cz + 1) as f32 * c);
            for cx in cx0..=cx1 {
                let (xl, xh) = (cx as f32 * c, (cx + 1) as f32 * c);
                let Some(clip) = clip_to_cell(tri, xl, xh, zl, zh) else {
                    continue;
                };
                let lo = ((clip.y.0 / y_step).floor() as i64).clamp(-Q_MAX, Q_MAX);
                let hi = ((clip.y.1 / y_step).ceil() as i64).clamp(-Q_MAX, Q_MAX);
                let rect = SubRect {
                    x0: idx(clip.x.0, xl),
                    x1: idx(clip.x.1, xl),
                    z0: idx(clip.z.0, zl),
                    z1: idx(clip.z.1, zl),
                };
                let tile = (cx >> TILE_SHIFT, cz >> TILE_SHIFT);
                let local = ((cz & 15) * TILE as i64 + (cx & 15)) as u64;
                self.tiles.entry(tile).or_default().push(
                    Rec {
                        cell: local,
                        lo,
                        hi,
                        rect,
                    },
                    gap,
                );
            }
        }
    }
}

/// Accumulates triangles; see the module docs.
pub struct OccluderBuilder {
    params: BuildParams,
    layers: Vec<LayerAcc>,
    terrain: Option<HeightfieldAcc>,
    /// Terrain triangles the heightfield refused (off the lattice).
    terrain_fallback: u64,
    /// Coverage blocks (edge `block`) touched by a floor-like triangle, or
    /// by a coverage triangle once [`Self::add_coverage_triangle`] was used.
    floor_blocks: HashSet<(i64, i64)>,
    /// Coverage comes from [`Self::add_coverage_triangle`] (a navmesh)
    /// rather than from floor-like collision triangles.
    external_coverage: bool,
    block: f32,
    hash: u64,
    triangles: u64,
    label: String,
}

impl OccluderBuilder {
    /// A builder with `params`; `label` is stored in the file (map name and
    /// anything a reader should know about the build).
    pub fn new(params: BuildParams, label: impl Into<String>) -> Result<Self, BuildError> {
        let positive = |v: f32| v.is_finite() && v > 0.0;
        if !positive(params.cell) {
            return Err(BuildError::BadParams("cell must be > 0"));
        }
        if params.terrain_pitch.is_some_and(|c| !positive(c)) {
            return Err(BuildError::BadParams("terrain_pitch must be > 0"));
        }
        if !positive(params.y_step) {
            return Err(BuildError::BadParams("y_step must be > 0"));
        }
        if !(params.merge_gap.is_finite() && params.merge_gap >= 0.0) {
            return Err(BuildError::BadParams("merge_gap must be >= 0"));
        }
        if params.margin.is_some_and(|m| !(m.is_finite() && m >= 0.0)) {
            return Err(BuildError::BadParams("margin must be >= 0"));
        }
        let layers = vec![LayerAcc::new(LayerKind::Geometry, params.cell)];
        Ok(Self {
            terrain: params.terrain_pitch.map(HeightfieldAcc::new),
            terrain_fallback: 0,
            block: params.margin.unwrap_or(0.0).max(1.0),
            params,
            layers,
            floor_blocks: HashSet::new(),
            external_coverage: false,
            hash: 0xcbf2_9ce4_8422_2325,
            triangles: 0,
            label: label.into(),
        })
    }

    /// Triangles accepted so far.
    pub fn triangle_count(&self) -> u64 {
        self.triangles
    }

    /// Terrain triangles that were not on the lattice and went to the
    /// geometry layer instead.
    pub fn terrain_fallback_count(&self) -> u64 {
        self.terrain_fallback
    }

    /// Add one triangle, in BigWorld metres. Non-finite triangles are
    /// dropped.
    pub fn add_triangle(&mut self, tri: &Triangle, source: Source) {
        if !tri.iter().flatten().all(|v| v.is_finite()) {
            return;
        }
        self.triangles += 1;
        for v in tri.iter().flatten() {
            for b in v.to_bits().to_le_bytes() {
                self.hash ^= b as u64;
                self.hash = self.hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        self.hash ^= source as u64;
        if self.params.margin.is_some()
            && !self.external_coverage
            && is_floor_like(tri, self.params.max_floor_slope_deg)
        {
            self.mark_floor(tri);
        }
        if source == Source::Terrain {
            if let Some(hf) = &mut self.terrain {
                if hf.add(tri) {
                    return;
                }
                self.terrain_fallback += 1;
            }
        }
        let gap = (self.params.merge_gap / self.params.y_step).round() as i64;
        self.layers[0].rasterise(tri, self.params.y_step, gap);
    }

    /// Take the coverage mask from walkable polygons (a navmesh, fan
    /// triangulated) instead of from floor-like collision triangles. Call it
    /// before the first [`Self::add_triangle`]: once used, collision
    /// triangles no longer mark coverage. Everything within the margin of
    /// the walkable surface is kept, so what a ray between two standing
    /// points can reach is still inside; geometry further than the margin
    /// from any floor (a mountainside, a rooftop no one can reach) is not.
    pub fn add_coverage_triangle(&mut self, tri: &Triangle) {
        if !tri.iter().flatten().all(|v| v.is_finite()) {
            return;
        }
        self.external_coverage = true;
        if self.params.margin.is_some() {
            self.mark_floor(tri);
        }
    }

    fn mark_floor(&mut self, tri: &Triangle) {
        let b = self.block;
        let (mut x0, mut x1, mut z0, mut z1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for v in tri {
            x0 = x0.min(v[0]);
            x1 = x1.max(v[0]);
            z0 = z0.min(v[2]);
            z1 = z1.max(v[2]);
        }
        for bz in (z0 / b).floor() as i64..=(z1 / b).floor() as i64 {
            for bx in (x0 / b).floor() as i64..=(x1 / b).floor() as i64 {
                self.floor_blocks.insert((bx, bz));
            }
        }
    }

    /// Finish the build: merge, trim to coverage and index.
    pub fn finish(mut self) -> Result<Occluder, BuildError> {
        if self.triangles == 0 {
            return Err(BuildError::Empty);
        }
        let gap = (self.params.merge_gap / self.params.y_step).round() as i64;
        let (mut qmin, mut qmax) = (i64::MAX, i64::MIN);
        for layer in &mut self.layers {
            for acc in layer.tiles.values_mut() {
                acc.compact(gap);
                for &r in &acc.recs {
                    let r = Rec::unpack(r);
                    qmin = qmin.min(r.lo);
                    qmax = qmax.max(r.hi);
                }
            }
        }
        if qmin > qmax {
            // Only heightfield terrain: the span quantisation is unused.
            qmin = 0;
            qmax = 0;
        }
        if qmax - qmin > u16::MAX as i64 {
            return Err(BuildError::YRangeTooLarge(qmax - qmin));
        }
        let covered: Vec<HashSet<(i64, i64)>> = self
            .layers
            .iter()
            .map(|l| self.covered_tiles(l.cell, l.tiles.keys().copied()))
            .collect();
        let heightfield = self.terrain.take().and_then(|hf| {
            let keys: Vec<(i64, i64)> = hf.tile_keys().collect();
            let cov = self.covered_tiles(hf.pitch(), keys.into_iter());
            hf.finish(&cov)
        });
        let layers = self
            .layers
            .into_iter()
            .zip(covered)
            .filter_map(|(acc, cov)| finish_layer(acc, cov, qmin))
            // A layer with nothing solid only repeats the coverage the
            // other layer already carries (both come from one mask).
            .filter(|l| !l.heads.is_empty())
            .collect::<Vec<_>>();
        if layers.is_empty() && heightfield.is_none() {
            return Err(BuildError::Empty);
        }
        let mut occ = Occluder {
            y_base: qmin as f32 * self.params.y_step,
            y_step: self.params.y_step,
            source_hash: self.hash,
            label: self.label,
            layers,
            heightfield,
            content_hash: 0,
            short_hash: String::new(),
        };
        let h = crate::grid::fnv1a64(&format::encode(&occ));
        occ.set_content_hash(h);
        Ok(occ)
    }

    /// The tiles of `layer` inside the coverage mask: with no margin, every
    /// tile holding geometry; otherwise every tile within `margin` of a
    /// floor-like block, whether or not it holds geometry.
    fn covered_tiles(
        &self,
        cell: f32,
        stored: impl Iterator<Item = (i64, i64)>,
    ) -> HashSet<(i64, i64)> {
        let Some(margin) = self.params.margin else {
            return stored.collect();
        };
        let tile_w = cell * TILE as f32;
        let mut out = HashSet::new();
        for &(bx, bz) in &self.floor_blocks {
            let (x0, z0) = (
                bx as f32 * self.block - margin,
                bz as f32 * self.block - margin,
            );
            let (x1, z1) = (
                (bx + 1) as f32 * self.block + margin,
                (bz + 1) as f32 * self.block + margin,
            );
            for tz in (z0 / tile_w).floor() as i64..=(z1 / tile_w).floor() as i64 {
                for tx in (x0 / tile_w).floor() as i64..=(x1 / tile_w).floor() as i64 {
                    out.insert((tx, tz));
                }
            }
        }
        out
    }
}

/// Lay one layer's covered tiles out as a [`Layer`]. `None` when nothing is
/// covered.
fn finish_layer(acc: LayerAcc, covered: HashSet<(i64, i64)>, qmin: i64) -> Option<Layer> {
    let (mut tx0, mut tx1, mut tz0, mut tz1) = (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
    for &(tx, tz) in &covered {
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
    let mut slots = vec![SLOT_UNCOVERED; tiles_x as usize * tiles_z as usize];
    let mut heads = Vec::new();
    let mut cell_end = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut tiles = acc.tiles;
    for tz in tz0..=tz1 {
        for tx in tx0..=tx1 {
            if !covered.contains(&(tx, tz)) {
                continue;
            }
            let slot = ((tz - tz0) * tiles_x as i64 + (tx - tx0)) as usize;
            let recs = tiles.remove(&(tx, tz)).map(|a| a.recs).unwrap_or_default();
            if recs.is_empty() {
                slots[slot] = SLOT_EMPTY;
                continue;
            }
            let mut per_cell: Vec<Vec<Span>> = vec![Vec::new(); TILE_CELLS];
            for r in recs {
                let r = Rec::unpack(r);
                per_cell[r.cell as usize].push(Span {
                    lo: (r.lo - qmin) as u16,
                    hi: (r.hi - qmin) as u16,
                    rect: r.rect.pack(),
                });
            }
            for c in &mut per_cell {
                cap_spans(c);
            }
            let first = per_cell[0].len();
            let uniform = per_cell.iter().all(|c| c.len() == first);
            let span_base = spans.len() as u32;
            let mut head = TileHead {
                span_base,
                uniform: if uniform { first as u8 } else { VARIABLE },
                ends: 0,
            };
            if !uniform {
                head.ends = cell_end.len() as u32;
            }
            let mut end = 0u16;
            for c in &per_cell {
                spans.extend_from_slice(c);
                end += c.len() as u16;
                if !uniform {
                    cell_end.push(end);
                }
            }
            slots[slot] = heads.len() as u32;
            heads.push(head);
        }
    }
    Some(Layer {
        kind: acc.kind,
        cell: acc.cell,
        origin: [
            (tx0 * TILE as i64) as f32 * acc.cell,
            (tz0 * TILE as i64) as f32 * acc.cell,
        ],
        tiles_x,
        tiles_z,
        slots,
        heads,
        cell_end,
        spans,
    })
}

/// Merge the closest neighbours (by `lo`) until the cell fits the on-disk
/// count byte. Merging can only grow solid space.
fn cap_spans(c: &mut Vec<Span>) {
    while c.len() > MAX_SPANS_PER_CELL {
        let i = (0..c.len() - 1)
            .min_by_key(|&i| c[i + 1].lo.saturating_sub(c[i].hi))
            .unwrap_or(0);
        let next = c.remove(i + 1);
        let r = SubRect::unpack(c[i].rect).union(SubRect::unpack(next.rect));
        c[i] = Span {
            lo: c[i].lo.min(next.lo),
            hi: c[i].hi.max(next.hi),
            rect: r.pack(),
        };
    }
}

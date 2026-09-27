//! Span accumulation: per-cell records, merging, and laying a finished
//! layer out as a [`Layer`].

use std::collections::{HashMap, HashSet};

use crate::grid::{
    Layer, LayerKind, Span, SubRect, TileHead, SLOT_EMPTY, SLOT_UNCOVERED, SUB, TILE, TILE_CELLS,
    TILE_SHIFT, VARIABLE,
};
use crate::raster::{clip_to_cell, Triangle};

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
pub(super) struct Rec {
    pub(super) cell: u64,
    pub(super) lo: i64,
    pub(super) hi: i64,
    pub(super) rect: SubRect,
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

    pub(super) fn unpack(r: u64) -> Self {
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
pub(super) struct TileAcc {
    /// Packed [`Rec`]s.
    pub(super) recs: Vec<u64>,
    /// Length after the last compaction.
    clean: usize,
}

impl TileAcc {
    /// Add a record. While accumulating, only exact duplicates are folded:
    /// the phantom-volume merge is order-dependent, and the extractor's
    /// triangle order varies between runs, so merging waits for
    /// [`Self::compact`] on the complete, sorted set and the output is the
    /// same whatever order the triangles came in.
    fn push(&mut self, rec: Rec) {
        self.recs.push(rec.pack());
        if self.recs.len() > 2 * self.clean + 1024 {
            self.recs.sort_unstable();
            self.recs.dedup();
            self.clean = self.recs.len();
        }
    }

    /// Sort and merge the records in place.
    pub(super) fn compact(&mut self, gap: i64) {
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

pub(super) struct LayerAcc {
    pub(super) kind: LayerKind,
    pub(super) cell: f32,
    pub(super) tiles: HashMap<(i64, i64), TileAcc>,
}

impl LayerAcc {
    pub(super) fn new(kind: LayerKind, cell: f32) -> Self {
        Self {
            kind,
            cell,
            tiles: HashMap::new(),
        }
    }

    /// Rasterise `tri` into its cells, only inside the `allowed` tiles
    /// when given (the trimmed coverage: a giant triangle is clipped to it,
    /// instead of filling every cell of its bounding box).
    pub(super) fn rasterise(
        &mut self,
        tri: &Triangle,
        y_step: f32,
        allowed: Option<&HashSet<(i64, i64)>>,
    ) {
        let (mut x0, mut x1, mut z0, mut z1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for v in tri {
            x0 = x0.min(v[0]);
            x1 = x1.max(v[0]);
            z0 = z0.min(v[2]);
            z1 = z1.max(v[2]);
        }
        let c = self.cell;
        let (cx0, cx1) = ((x0 / c).floor() as i64, (x1 / c).floor() as i64);
        let (cz0, cz1) = ((z0 / c).floor() as i64, (z1 / c).floor() as i64);
        let t = TILE as i64;
        for tz in cz0.div_euclid(t)..=cz1.div_euclid(t) {
            for tx in cx0.div_euclid(t)..=cx1.div_euclid(t) {
                if allowed.is_some_and(|a| !a.contains(&(tx, tz))) {
                    continue;
                }
                let (az, bz) = (cz0.max(tz * t), cz1.min(tz * t + t - 1));
                let (ax, bx) = (cx0.max(tx * t), cx1.min(tx * t + t - 1));
                for cz in az..=bz {
                    for cx in ax..=bx {
                        self.rasterise_cell(tri, cx, cz, y_step);
                    }
                }
            }
        }
    }

    fn rasterise_cell(&mut self, tri: &Triangle, cx: i64, cz: i64, y_step: f32) {
        let c = self.cell;
        let sub = c / SUB as f32;
        // Sub-cell index of a coordinate inside the cell starting at `base`,
        // clamped; min and max both round down, so the rectangle (which
        // runs to the far edge of its last sub-cell) rounds outwards.
        let idx =
            |v: f32, base: f32| ((v - base) / sub).floor().clamp(0.0, (SUB - 1) as f32) as u16;
        let (zl, zh) = (cz as f32 * c, (cz + 1) as f32 * c);
        let (xl, xh) = (cx as f32 * c, (cx + 1) as f32 * c);
        let Some(clip) = clip_to_cell(tri, xl, xh, zl, zh) else {
            return;
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
        self.tiles.entry(tile).or_default().push(Rec {
            cell: local,
            lo,
            hi,
            rect,
        });
    }
}

/// Lay one layer's covered tiles out as a [`Layer`]. `None` when nothing is
/// covered.
pub(super) fn finish_layer(
    acc: LayerAcc,
    covered: HashSet<(i64, i64)>,
    qmin: i64,
) -> Option<Layer> {
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

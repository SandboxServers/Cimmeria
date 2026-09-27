//! The in-memory occluder: layers of tiled column grids of solid Y spans.
//!
//! Each [`Layer`] covers an axis-aligned XZ rectangle, cut into square
//! tiles of [`TILE`] x [`TILE`] cells. A tile is one of:
//!
//! - **uncovered** — trimmed away at build time (no walkable surface within
//!   the margin) or never touched by geometry. A query endpoint here is off
//!   the grid; a ray *passing through* one sees nothing there.
//! - **covered, empty** — inside the coverage mask, but no solid geometry.
//! - **stored** — each of its cells holds a list of solid [`Span`]s sorted
//!   by `lo`: a Y range `[lo, hi]`, quantised to [`Occluder::y_step`] above
//!   [`Occluder::y_base`] (`lo` rounded down, `hi` rounded up, so a span
//!   never shrinks), and the XZ rectangle of the cell the solid part
//!   occupies, on a [`SUB`] x [`SUB`] sub-grid (rounded outwards). Spans in
//!   one cell may overlap in Y when their rectangles differ: a thin floor
//!   under the whole cell and a wall along one edge of it stay two spans,
//!   so the wall does not fill the cell.
//!
//! A tile whose cells all hold the same number of spans (the common case:
//! a terrain tile has exactly one per cell) stores no per-cell index, which
//! is what keeps an outdoor map's ground layer at 4 bytes per cell.

/// Cells per tile side. A power of two so the tile / local split is a shift.
pub const TILE: usize = 16;
pub(crate) const TILE_SHIFT: u32 = 4;
pub(crate) const TILE_MASK: i64 = (TILE as i64) - 1;
pub(crate) const TILE_CELLS: usize = TILE * TILE;

/// `slots` value: outside the coverage mask.
pub(crate) const SLOT_UNCOVERED: u32 = u32::MAX;
/// `slots` value: covered, no solid geometry.
pub(crate) const SLOT_EMPTY: u32 = u32::MAX - 1;
/// `TileHead::uniform` value: cells hold different span counts, see `ends`.
pub(crate) const VARIABLE: u8 = u8::MAX;
/// Sub-cells per cell side for a span's XZ rectangle.
pub const SUB: u16 = 16;

/// One solid span of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Span {
    /// Bottom, Y quanta.
    pub lo: u16,
    /// Top, Y quanta.
    pub hi: u16,
    /// XZ rectangle inside the cell: [`SubRect`] packed.
    pub rect: u16,
}

/// A span's XZ rectangle on the cell's sub-grid: inclusive sub-cell index
/// ranges `x0..=x1`, `z0..=z1`, each in `0..SUB`. Packed as four nibbles
/// `x0 x1 z0 z1`, high to low.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubRect {
    pub x0: u16,
    pub x1: u16,
    pub z0: u16,
    pub z1: u16,
}

impl SubRect {
    /// The whole cell.
    pub const FULL: Self = Self {
        x0: 0,
        x1: SUB - 1,
        z0: 0,
        z1: SUB - 1,
    };

    pub fn pack(self) -> u16 {
        (self.x0 << 12) | (self.x1 << 8) | (self.z0 << 4) | self.z1
    }

    pub fn unpack(v: u16) -> Self {
        Self {
            x0: v >> 12,
            x1: (v >> 8) & 0xF,
            z0: (v >> 4) & 0xF,
            z1: v & 0xF,
        }
    }

    /// Sub-cells covered.
    pub fn area(self) -> u32 {
        (self.x1 - self.x0 + 1) as u32 * (self.z1 - self.z0 + 1) as u32
    }

    pub fn union(self, o: Self) -> Self {
        Self {
            x0: self.x0.min(o.x0),
            x1: self.x1.max(o.x1),
            z0: self.z0.min(o.z0),
            z1: self.z1.max(o.z1),
        }
    }

    /// Whether the fields form a valid rectangle.
    pub fn is_valid(self) -> bool {
        self.x0 <= self.x1 && self.z0 <= self.z1 && self.x1 < SUB && self.z1 < SUB
    }
}

/// What a layer's triangles came from. Decides nothing at query time; it is
/// recorded so a report can say which layer a blocked ray hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    /// StaticMesh and BSP geometry (and terrain, when not split out).
    Geometry = 0,
    /// The terrain heightfield ([`crate::heightfield`]).
    Terrain = 1,
}

impl LayerKind {
    pub(crate) fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Geometry),
            1 => Some(Self::Terrain),
            _ => None,
        }
    }

    /// Stable label for reports and logs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Geometry => "geometry",
            Self::Terrain => "terrain",
        }
    }
}

/// One stored tile's index into the layer's span pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileHead {
    /// Index of the tile's first span in [`Layer::spans`].
    pub span_base: u32,
    /// Spans per cell when every cell holds the same number, else
    /// [`VARIABLE`].
    pub uniform: u8,
    /// For a [`VARIABLE`] tile, the index of its first entry in
    /// [`Layer::cell_end`] (`TILE_CELLS` entries: the exclusive end of each
    /// cell's spans, relative to `span_base`). Unused otherwise.
    pub ends: u32,
}

/// One tiled column grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub(crate) kind: LayerKind,
    /// Cell edge, metres.
    pub(crate) cell: f32,
    /// World XZ of the corner of tile (0, 0).
    pub(crate) origin: [f32; 2],
    pub(crate) tiles_x: u32,
    pub(crate) tiles_z: u32,
    /// `tiles_x * tiles_z`, row-major by z: [`SLOT_UNCOVERED`],
    /// [`SLOT_EMPTY`], or an index into `heads`.
    pub(crate) slots: Vec<u32>,
    pub(crate) heads: Vec<TileHead>,
    pub(crate) cell_end: Vec<u16>,
    pub(crate) spans: Vec<Span>,
}

/// What one cell of a layer holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cell<'a> {
    /// Outside the layer's rectangle.
    Outside,
    /// Inside the rectangle, in an uncovered tile.
    Uncovered,
    /// Covered; the spans may be empty.
    Spans(&'a [Span]),
}

impl Layer {
    /// What this layer's triangles came from.
    pub fn kind(&self) -> LayerKind {
        self.kind
    }

    /// Cell edge in metres.
    pub fn cell_size(&self) -> f32 {
        self.cell
    }

    /// Grid dimensions in cells, `(x, z)`.
    pub fn dims(&self) -> (u32, u32) {
        (self.tiles_x * TILE as u32, self.tiles_z * TILE as u32)
    }

    /// World-space XZ rectangle `(min, max)` the layer spans.
    pub fn bounds(&self) -> ([f32; 2], [f32; 2]) {
        let w = self.tiles_x as f32 * TILE as f32 * self.cell;
        let h = self.tiles_z as f32 * TILE as f32 * self.cell;
        (self.origin, [self.origin[0] + w, self.origin[1] + h])
    }

    /// Tiles holding at least one span.
    pub fn stored_tiles(&self) -> usize {
        self.heads.len()
    }

    /// Tiles inside the coverage mask (stored or empty).
    pub fn covered_tiles(&self) -> usize {
        self.slots.iter().filter(|&&s| s != SLOT_UNCOVERED).count()
    }

    /// Solid spans across every cell.
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Bytes this layer holds on the heap.
    pub fn ram_bytes(&self) -> usize {
        self.slots.len() * 4
            + self.heads.len() * std::mem::size_of::<TileHead>()
            + self.cell_end.len() * 2
            + self.spans.len() * std::mem::size_of::<Span>()
    }

    /// Integer cell coordinates of world `(x, z)`.
    #[inline]
    pub(crate) fn cell_of(&self, x: f32, z: f32) -> (i64, i64) {
        (
            ((x - self.origin[0]) / self.cell).floor() as i64,
            ((z - self.origin[1]) / self.cell).floor() as i64,
        )
    }

    /// The slot of the tile containing cell `(cx, cz)`, or `None` outside
    /// the rectangle.
    #[inline]
    fn slot(&self, cx: i64, cz: i64) -> Option<u32> {
        if cx < 0 || cz < 0 {
            return None;
        }
        let (tx, tz) = (cx >> TILE_SHIFT, cz >> TILE_SHIFT);
        if tx >= self.tiles_x as i64 || tz >= self.tiles_z as i64 {
            return None;
        }
        Some(self.slots[(tz * self.tiles_x as i64 + tx) as usize])
    }

    /// The spans of cell `(cx, cz)`.
    #[inline]
    pub(crate) fn cell(&self, cx: i64, cz: i64) -> Cell<'_> {
        let slot = match self.slot(cx, cz) {
            None => return Cell::Outside,
            Some(SLOT_UNCOVERED) => return Cell::Uncovered,
            Some(SLOT_EMPTY) => return Cell::Spans(&[]),
            Some(s) => s,
        };
        let head = self.heads[slot as usize];
        let local = ((cz & TILE_MASK) * TILE as i64 + (cx & TILE_MASK)) as usize;
        let (start, end) = if head.uniform != VARIABLE {
            let k = head.uniform as usize;
            (local * k, local * k + k)
        } else {
            let ends = &self.cell_end[head.ends as usize..head.ends as usize + TILE_CELLS];
            let start = if local == 0 {
                0
            } else {
                ends[local - 1] as usize
            };
            (start, ends[local] as usize)
        };
        let base = head.span_base as usize;
        Cell::Spans(&self.spans[base + start..base + end])
    }

    /// Whether world `(x, z)` lies in a covered tile of this layer.
    pub fn covers(&self, x: f32, z: f32) -> bool {
        let (cx, cz) = self.cell_of(x, z);
        matches!(self.cell(cx, cz), Cell::Spans(_))
    }
}

/// A space's occluder: every layer plus the shared Y quantisation.
#[derive(Debug, Clone, PartialEq)]
pub struct Occluder {
    pub(crate) y_base: f32,
    pub(crate) y_step: f32,
    pub(crate) source_hash: u64,
    pub(crate) label: String,
    pub(crate) layers: Vec<Layer>,
    /// Lattice-aligned terrain, when the build had any.
    pub(crate) heightfield: Option<crate::heightfield::Heightfield>,
    /// FNV-1a 64 of the encoded file, or of the encoding a freshly built
    /// occluder would have. Set by the loader and by the builder, through
    /// [`Self::set_content_hash`].
    pub(crate) content_hash: u64,
    /// [`Self::content_hash`] as 8 hex digits, kept so a log line does not
    /// format it per query.
    pub(crate) short_hash: String,
}

impl Occluder {
    /// World Y of quantum 0.
    pub fn y_base(&self) -> f32 {
        self.y_base
    }

    /// Height of one Y quantum, metres.
    pub fn y_step(&self) -> f32 {
        self.y_step
    }

    /// Hash of the triangles the builder consumed; two builds from the same
    /// extraction agree.
    pub fn source_hash(&self) -> u64 {
        self.source_hash
    }

    /// Free-form build label (map name and build parameters).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The span layers (today one: StaticMesh, BSP, and terrain that is not
    /// on the lattice).
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// The terrain heightfield, if any.
    pub fn heightfield(&self) -> Option<&crate::heightfield::Heightfield> {
        self.heightfield.as_ref()
    }

    /// Bytes held on the heap across every layer.
    pub fn ram_bytes(&self) -> usize {
        self.layers.iter().map(Layer::ram_bytes).sum::<usize>()
            + self.heightfield.as_ref().map_or(0, |h| h.ram_bytes())
            + self.label.len()
    }

    /// Hash of the encoded bytes.
    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    /// The first 8 hex digits of [`Self::content_hash`], for log lines
    /// (the `.nav` fingerprint uses the same shape).
    pub fn short_hash(&self) -> &str {
        &self.short_hash
    }

    pub(crate) fn set_content_hash(&mut self, h: u64) {
        self.content_hash = h;
        self.short_hash = format!("{h:016x}")[..8].to_string();
    }

    /// Whether world `(x, z)` is inside the coverage of any layer. A query
    /// endpoint outside it gets [`crate::Sight::OffGrid`].
    pub fn covers(&self, x: f32, z: f32) -> bool {
        self.layers.iter().any(|l| l.covers(x, z))
            || self.heightfield.as_ref().is_some_and(|h| h.covers(x, z))
    }

    /// A span's Y range in metres.
    #[inline]
    pub(crate) fn span_metres(&self, s: Span) -> (f32, f32) {
        (
            self.y_base + s.lo as f32 * self.y_step,
            self.y_base + s.hi as f32 * self.y_step,
        )
    }

    /// The solid spans (metres) of the column containing world `(x, z)` in
    /// every covered layer, lowest first per layer. For tests and tools.
    pub fn column(&self, x: f32, z: f32) -> Vec<(LayerKind, f32, f32)> {
        let mut out = Vec::new();
        for l in &self.layers {
            let (cx, cz) = l.cell_of(x, z);
            if let Cell::Spans(spans) = l.cell(cx, cz) {
                for &s in spans {
                    let (lo, hi) = self.span_metres(s);
                    out.push((l.kind, lo, hi));
                }
            }
        }
        if let Some(h) = &self.heightfield {
            let (px, pz) = h.patch_of(x, z);
            let (tris, flags) = h.patch_triangles(px, pz);
            if flags & (crate::heightfield::HAS_A | crate::heightfield::HAS_B) != 0 {
                let ys = tris.iter().flatten().map(|v| v[1]);
                let lo = ys.clone().fold(f32::INFINITY, f32::min);
                let hi = ys.fold(f32::NEG_INFINITY, f32::max);
                out.push((LayerKind::Terrain, lo, hi));
            }
        }
        out
    }
}

/// FNV-1a 64. Enough to fingerprint a file; not a security boundary.
pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

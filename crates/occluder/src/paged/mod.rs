//! The paged occluder: a world cut into square pages, each kept compressed
//! in memory and unpacked only while someone is near it (NA27, D-NA13).
//!
//! A whole outdoor world unpacked is 100-300 MB. Players only ever stand in
//! a few pages at once, and NPCs only look where players are. So:
//!
//! - the `.occ` file is a page table plus one compressed blob per page
//!   ([`file`]); loading it decompresses nothing;
//! - [`PagedOccluder::retain_near`] (called by the cell each second with
//!   every player position) unpacks the pages within a radius of any player
//!   and evicts the rest;
//! - a query that touches a page that is still packed unpacks it on the
//!   spot. That costs about a millisecond per page, so it is cheaper than
//!   answering `unknown`, and the next `retain_near` evicts it again if no
//!   player is near.
//!
//! A query answers exactly what the unpaged [`Occluder`] would: each page
//! tests its own stretch of the segment, and a page is a verbatim window of
//! the unpaged tiles ([`split`]).

mod file;
mod split;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::build::BuildError;
use crate::format::{self, OccluderError};
use crate::grid::Occluder;
use crate::query::Sight;

pub use file::{PAGED_MAGIC, PAGED_VERSION};

/// The page grid: page `(px, pz)` covers `[px * size, (px + 1) * size)` in
/// world X and Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGrid {
    pub size: f32,
    pub px0: i32,
    pub pz0: i32,
    pub nx: u32,
    pub nz: u32,
}

impl PageGrid {
    /// The page index of world `(x, z)`, or `None` outside the grid.
    pub fn index_of(&self, x: f32, z: f32) -> Option<u32> {
        let px = (x / self.size).floor() as i64 - self.px0 as i64;
        let pz = (z / self.size).floor() as i64 - self.pz0 as i64;
        (px >= 0 && pz >= 0 && px < self.nx as i64 && pz < self.nz as i64)
            .then(|| pz as u32 * self.nx + px as u32)
    }
}

/// The default page edge, metres: 128 geometry cells at 0.5 m.
pub const DEFAULT_PAGE_SIZE: f32 = 64.0;

/// Encode `occ` as a paged `.occ` with pages of `page_size` metres.
pub fn encode_paged(occ: &Occluder, page_size: f32) -> Result<Vec<u8>, BuildError> {
    let (grid, pages) = split::split(occ, page_size)?;
    let blobs: Vec<(u32, Vec<u8>, usize)> = pages
        .iter()
        .map(|(idx, p)| (*idx, format::encode(p), p.ram_bytes()))
        .collect();
    Ok(file::encode(&grid, occ.source_hash, &occ.label, &blobs))
}

/// What [`PagedOccluder::retain_near`] did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Residency {
    /// Pages it unpacked.
    pub unpacked: Vec<u32>,
    /// Pages it evicted.
    pub evicted: Vec<u32>,
    /// Pages resident afterwards, and their bytes.
    pub resident_pages: usize,
    pub resident_bytes: usize,
}

/// Running counters since load.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PageStats {
    /// Pages in the file.
    pub pages: usize,
    /// Bytes of the file held in memory (every page, packed).
    pub packed_bytes: usize,
    /// Pages unpacked right now, and their bytes.
    pub resident_pages: usize,
    pub resident_bytes: usize,
    /// Unpacks since load, and how many of them a query forced.
    pub unpacks: u64,
    pub query_unpacks: u64,
    pub evictions: u64,
    /// Total and worst single unpack time, microseconds.
    pub unpack_us_total: u64,
    pub unpack_us_max: u64,
}

struct Resident {
    occ: Arc<Occluder>,
    bytes: usize,
}

#[derive(Default)]
struct Cache {
    resident: HashMap<u32, Resident>,
    stats: PageStats,
}

/// A loaded paged occluder; see the module docs.
pub struct PagedOccluder {
    file: file::PagedFile,
    /// Page index to its position in `file.pages`.
    lookup: HashMap<u32, usize>,
    short_hash: String,
    cache: Mutex<Cache>,
}

impl std::fmt::Debug for PagedOccluder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PagedOccluder")
            .field("label", &self.file.label)
            .field("grid", &self.file.grid)
            .field("pages", &self.file.pages.len())
            .finish()
    }
}

impl PagedOccluder {
    /// Parse a paged file held in memory. Nothing is unpacked.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, OccluderError> {
        let file = file::parse(bytes)?;
        let lookup = file
            .pages
            .iter()
            .enumerate()
            .map(|(i, (idx, _))| (*idx, i))
            .collect();
        let short_hash = format!("{:016x}", file.content_hash)[..8].to_string();
        let stats = PageStats {
            pages: file.pages.len(),
            packed_bytes: file.bytes.len(),
            ..PageStats::default()
        };
        Ok(Self {
            file,
            lookup,
            short_hash,
            cache: Mutex::new(Cache {
                resident: HashMap::new(),
                stats,
            }),
        })
    }

    /// Read a paged `.occ` file.
    pub fn load(path: &std::path::Path) -> Result<Self, OccluderError> {
        Self::from_bytes(std::fs::read(path)?)
    }

    /// The page grid.
    pub fn grid(&self) -> PageGrid {
        self.file.grid
    }

    /// Free-form build label.
    pub fn label(&self) -> &str {
        &self.file.label
    }

    /// Hash of the input triangles.
    pub fn source_hash(&self) -> u64 {
        self.file.source_hash
    }

    /// FNV-1a 64 of the file, as 8 hex digits.
    pub fn short_hash(&self) -> &str {
        &self.short_hash
    }

    /// Sum of every page's unpacked size: the RAM of the whole world
    /// resident at once.
    pub fn full_ram_bytes(&self) -> usize {
        self.file
            .pages
            .iter()
            .map(|(_, r)| r.ram_bytes as usize)
            .sum()
    }

    /// Counters and current residency.
    pub fn stats(&self) -> PageStats {
        self.lock().stats
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Cache> {
        self.cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The unpacked page `idx`, unpacking it now if needed. `None` for a page
    /// the file does not hold, or one that fails to decode.
    fn page(&self, idx: u32, by_query: bool) -> Option<Arc<Occluder>> {
        let mut cache = self.lock();
        if let Some(r) = cache.resident.get(&idx) {
            return Some(r.occ.clone());
        }
        let &(_, r) = self.file.pages.get(*self.lookup.get(&idx)?)?;
        let started = Instant::now();
        let occ = format::decode(self.file.blob(&r)).ok()?;
        let us = started.elapsed().as_micros() as u64;
        let bytes = occ.ram_bytes();
        let occ = Arc::new(occ);
        let st = &mut cache.stats;
        st.unpacks += 1;
        st.query_unpacks += u64::from(by_query);
        st.unpack_us_total += us;
        st.unpack_us_max = st.unpack_us_max.max(us);
        st.resident_pages += 1;
        st.resident_bytes += bytes;
        cache.resident.insert(
            idx,
            Resident {
                occ: occ.clone(),
                bytes,
            },
        );
        Some(occ)
    }

    /// Unpack every page within `radius` metres (XZ) of any of `points`
    /// and evict every other resident page.
    pub fn retain_near(&self, points: &[[f32; 2]], radius: f32) -> Residency {
        let g = self.file.grid;
        let mut wanted = std::collections::HashSet::new();
        for p in points {
            let (x0, x1) = (p[0] - radius, p[0] + radius);
            let (z0, z1) = (p[1] - radius, p[1] + radius);
            let (ax, bx) = ((x0 / g.size).floor() as i64, (x1 / g.size).floor() as i64);
            let (az, bz) = ((z0 / g.size).floor() as i64, (z1 / g.size).floor() as i64);
            for pz in az..=bz {
                for px in ax..=bx {
                    // Distance from the point to the page rectangle.
                    let cx = p[0].clamp(px as f32 * g.size, (px + 1) as f32 * g.size);
                    let cz = p[1].clamp(pz as f32 * g.size, (pz + 1) as f32 * g.size);
                    if (cx - p[0]).hypot(cz - p[1]) > radius {
                        continue;
                    }
                    let (lx, lz) = (px - g.px0 as i64, pz - g.pz0 as i64);
                    if lx < 0 || lz < 0 || lx >= g.nx as i64 || lz >= g.nz as i64 {
                        continue;
                    }
                    let idx = lz as u32 * g.nx + lx as u32;
                    if self.lookup.contains_key(&idx) {
                        wanted.insert(idx);
                    }
                }
            }
        }
        let mut out = Residency::default();
        {
            let mut cache = self.lock();
            let evict: Vec<u32> = cache
                .resident
                .keys()
                .filter(|k| !wanted.contains(k))
                .copied()
                .collect();
            for k in evict {
                if let Some(r) = cache.resident.remove(&k) {
                    cache.stats.resident_pages -= 1;
                    cache.stats.resident_bytes -= r.bytes;
                    cache.stats.evictions += 1;
                    out.evicted.push(k);
                }
            }
        }
        let mut want: Vec<u32> = wanted.into_iter().collect();
        want.sort_unstable();
        for idx in want {
            let resident = self.lock().resident.contains_key(&idx);
            if !resident && self.page(idx, false).is_some() {
                out.unpacked.push(idx);
            }
        }
        let st = self.stats();
        out.resident_pages = st.resident_pages;
        out.resident_bytes = st.resident_bytes;
        out.evicted.sort_unstable();
        out
    }

    /// Unpack every page (tests and measurement).
    pub fn unpack_all(&self) {
        let idx: Vec<u32> = self.file.pages.iter().map(|(i, _)| *i).collect();
        for i in idx {
            let _ = self.page(i, false);
        }
    }

    /// The solid spans under world `(x, z)`, as [`Occluder::column`].
    pub fn column(&self, x: f32, z: f32) -> Vec<(crate::LayerKind, f32, f32)> {
        self.file
            .grid
            .index_of(x, z)
            .and_then(|i| self.page(i, true))
            .map(|p| p.column(x, z))
            .unwrap_or_default()
    }

    /// Whether world `(x, z)` is covered.
    pub fn covers(&self, x: f32, z: f32) -> bool {
        self.file
            .grid
            .index_of(x, z)
            .and_then(|i| self.page(i, true))
            .is_some_and(|p| p.covers(x, z))
    }

    /// The eye-to-eye segment test, as [`Occluder::sight`].
    pub fn sight(&self, from: [f32; 3], to: [f32; 3]) -> Sight {
        if !(from.iter().chain(&to).all(|v| v.is_finite())) {
            return Sight::OffGrid;
        }
        if !self.covers(from[0], from[2]) || !self.covers(to[0], to[2]) {
            return Sight::OffGrid;
        }
        let g = self.file.grid;
        let len = ((to[0] - from[0]).powi(2) + (to[2] - from[2]).powi(2)).sqrt();
        let eps = if len > 1e-4 { 0.01 / len } else { 1.0 };
        let mut blocked = None;
        walk_pages(g.size, from, to, |px, pz, t0, t1| {
            let (lx, lz) = (px - g.px0 as i64, pz - g.pz0 as i64);
            if lx < 0 || lz < 0 || lx >= g.nx as i64 || lz >= g.nz as i64 {
                return false;
            }
            let Some(page) = self.page(lz as u32 * g.nx + lx as u32, true) else {
                return false;
            };
            // A hair of overlap into the neighbours: page and cell bounds
            // coincide, and rounding must not drop the sliver between them.
            // The overlap is outside this page's cells, so it adds nothing.
            let (t0, t1) = ((t0 - eps).max(0.0), (t1 + eps).min(1.0));
            if let Some((at, layer)) = page.hit_between(from, to, t0, t1) {
                blocked = Some(Sight::Blocked { at, layer });
                return true;
            }
            false
        });
        blocked.unwrap_or(Sight::Clear)
    }
}

/// Visit every page the XZ projection of `a -> b` crosses, in order, with
/// the segment parameter range inside it, until `visit` returns true.
fn walk_pages(
    size: f32,
    a: [f32; 3],
    b: [f32; 3],
    mut visit: impl FnMut(i64, i64, f32, f32) -> bool,
) {
    let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
    let cell = |v: f32| (v / size).floor() as i64;
    let (mut cx, mut cz) = (cell(a[0]), cell(a[2]));
    let (ex, ez) = (cell(b[0]), cell(b[2]));
    let axis = |d: f32, p: f32, c: i64| -> (i64, f32, f32) {
        if d > 0.0 {
            (1, size / d, ((c + 1) as f32 * size - p) / d)
        } else if d < 0.0 {
            (-1, size / -d, (c as f32 * size - p) / d)
        } else {
            (0, f32::INFINITY, f32::INFINITY)
        }
    };
    let (sx, tdx, mut tmx) = axis(dx, a[0], cx);
    let (sz, tdz, mut tmz) = axis(dz, a[2], cz);
    let mut t = 0.0f32;
    let steps = (ex - cx).abs() + (ez - cz).abs() + 2;
    for _ in 0..=steps {
        let next = tmx.min(tmz).min(1.0);
        if visit(cx, cz, t, next) || next >= 1.0 {
            return;
        }
        if tmx < tmz {
            cx += sx;
            t = tmx;
            tmx += tdx;
        } else {
            cz += sz;
            t = tmz;
            tmz += tdz;
        }
    }
}

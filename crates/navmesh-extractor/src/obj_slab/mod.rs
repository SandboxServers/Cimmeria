//! Query the *source* geometry inside a small box of BigWorld space.
//!
//! When [`crate::nav_components::gaps`] says two walkable components come
//! within 1.4 m of each other at `(280, 43, 875)`, the next question is what
//! is actually there — a doorway narrower than the erosion budget, a step
//! over `agentClimb`, a 60-degree ramp, a wall, or nothing at all because the
//! mesh was never extracted. Answering it means going back to the chunk OBJs
//! the navmesh was built from and measuring.
//!
//! This module reads those OBJs and answers three questions about a box:
//!
//! - [`Slab::column`] — every near-horizontal surface directly above or below
//!   a point, in height order. Floors, steps, landings and ceilings.
//! - [`Slab::occupancy`] + [`Occupancy::free_runs`] — where a height band is
//!   blocked by wall-like geometry, and how wide the clear openings through
//!   it are. This is the doorway-width measurement.
//! - [`Slab::slope_profile`] — how much of the surface area in the box is
//!   walkable at a given slope limit, and how much is just too steep.
//!
//! # Coordinates
//!
//! The chunk OBJs carry UE3 centimetres with Y and Z swapped: a `v` line is
//! `v <ue.x> <ue.z> <ue.y>`. `Mesh::loadOBJ` (`mesh.cpp:106-108`) then reads
//! `bw = (col2, col1, col0) / 100`. Everything this module returns is already
//! in BigWorld metres, so it can be compared with `.nav` coordinates
//! directly. See `docs/engine/navmesh-build-pipeline.md` §1.
//!
//! # Chunk pre-filter
//!
//! A 400 MB scan per query is avoidable: a chunk's id encodes its grid cell
//! (low `u16` → BW x index, high `u16` → BW z index, 100 m per cell), so
//! chunks that cannot touch any requested box are skipped without being
//! opened. Actors do overhang their owning chunk, so the test is widened by
//! [`SlabSet::margin`] (default 60 m) — raise it if a query comes back
//! suspiciously empty, and compare against a `margin` large enough to read
//! everything.

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::chunk_id::ChunkId;

/// One source triangle, in BigWorld metres.
#[derive(Debug, Clone, Copy)]
pub struct Tri {
    pub v: [[f32; 3]; 3],
}

impl Tri {
    /// Unit normal, right-hand rule over the stored winding.
    pub fn normal(&self) -> [f32; 3] {
        let (a, b, c) = (self.v[0], self.v[1], self.v[2]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len <= 1e-12 {
            [0.0, 0.0, 0.0]
        } else {
            [n[0] / len, n[1] / len, n[2] / len]
        }
    }

    /// Angle between the triangle's plane and horizontal, degrees, ignoring
    /// which way it faces. 0 = flat, 90 = vertical wall.
    pub fn tilt_degrees(&self) -> f32 {
        self.normal()[1].abs().clamp(0.0, 1.0).acos().to_degrees()
    }

    /// Area of the XZ projection — the footprint a floor covers.
    pub fn area_xz(&self) -> f32 {
        let (a, b, c) = (self.v[0], self.v[1], self.v[2]);
        ((b[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (b[2] - a[2])).abs() * 0.5
    }

    fn bmin(&self) -> [f32; 3] {
        let mut m = self.v[0];
        for p in &self.v[1..] {
            for k in 0..3 {
                m[k] = m[k].min(p[k]);
            }
        }
        m
    }

    fn bmax(&self) -> [f32; 3] {
        let mut m = self.v[0];
        for p in &self.v[1..] {
            for k in 0..3 {
                m[k] = m[k].max(p[k]);
            }
        }
        m
    }

    fn overlaps(&self, bmin: [f32; 3], bmax: [f32; 3]) -> bool {
        let lo = self.bmin();
        let hi = self.bmax();
        (0..3).all(|k| hi[k] >= bmin[k] && lo[k] <= bmax[k])
    }

    /// Height of the triangle's plane at `(x, z)`, or `None` when `(x, z)` is
    /// outside the XZ projection (or the triangle is vertical, so has no
    /// single height there).
    pub fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        let (a, b, c) = (self.v[0], self.v[1], self.v[2]);
        let denom = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
        if denom.abs() < 1e-9 {
            return None;
        }
        let l1 = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / denom;
        let l2 = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / denom;
        let l3 = 1.0 - l1 - l2;
        const EPS: f32 = -1e-4;
        if l1 < EPS || l2 < EPS || l3 < EPS {
            return None;
        }
        Some(l1 * a[1] + l2 * b[1] + l3 * c[1])
    }
}

/// A near-horizontal surface found directly under or over a query point.
#[derive(Debug, Clone, Copy)]
pub struct Surface {
    pub y: f32,
    /// Tilt from horizontal in degrees; 0 is a flat floor.
    pub tilt_degrees: f32,
    /// True when the triangle faces up in UE3's winding convention. SGW's
    /// cooked meshes are wound for a left-handed front face, so a floor top
    /// has `normal.y < 0` after the axis permutation — see
    /// `docs/engine/navmesh-build-pipeline.md` §1.3. Reported rather than
    /// filtered on, because BSP and terrain do not all agree.
    pub faces_up: bool,
}

/// The triangles of one query box.
#[derive(Debug, Clone)]
pub struct Slab {
    pub name: String,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
    pub tris: Vec<Tri>,
    /// Chunk stem → triangles contributed, so a gap can be attributed to a
    /// specific `.umap`.
    pub by_chunk: BTreeMap<String, usize>,
}

impl Slab {
    pub fn new(name: impl Into<String>, bmin: [f32; 3], bmax: [f32; 3]) -> Self {
        Self {
            name: name.into(),
            bmin,
            bmax,
            tris: Vec::new(),
            by_chunk: BTreeMap::new(),
        }
    }

    /// Box centred on `at` with half-extents `half`.
    pub fn around(name: impl Into<String>, at: [f32; 3], half: [f32; 3]) -> Self {
        Self::new(
            name,
            [at[0] - half[0], at[1] - half[1], at[2] - half[2]],
            [at[0] + half[0], at[1] + half[1], at[2] + half[2]],
        )
    }

    /// Every near-horizontal surface at `(x, z)`, lowest first.
    ///
    /// `max_tilt` decides what counts as a surface rather than a wall; 60
    /// degrees keeps ramps and stairs in while dropping wall skins that the
    /// point happens to graze.
    pub fn column(&self, x: f32, z: f32, max_tilt: f32) -> Vec<Surface> {
        let mut out: Vec<Surface> = self
            .tris
            .iter()
            .filter(|t| t.tilt_degrees() <= max_tilt)
            .filter_map(|t| {
                t.height_at(x, z).map(|y| Surface {
                    y,
                    tilt_degrees: t.tilt_degrees(),
                    faces_up: t.normal()[1] > 0.0,
                })
            })
            .collect();
        out.sort_by(|a, b| a.y.total_cmp(&b.y));
        out
    }

    /// Clearance above `y` at `(x, z)`: the distance to the next surface
    /// overhead, or `None` when nothing is above.
    pub fn headroom(&self, x: f32, z: f32, y: f32, max_tilt: f32) -> Option<f32> {
        self.column(x, z, max_tilt)
            .into_iter()
            .find(|s| s.y > y + 0.05)
            .map(|s| s.y - y)
    }

    /// Near-horizontal surface area bucketed by height, with each bucket's
    /// XZ extent.
    ///
    /// This is the "is there a staircase between these two storeys" query: a
    /// stair shows up as a run of small buckets at regular intervals between
    /// two large ones, and its `bmin`/`bmax` say where to look. An empty band
    /// between two floors means there is no vertical connection there at all.
    pub fn level_histogram(&self, bucket: f32, max_tilt: f32) -> Vec<Level> {
        let bucket = bucket.max(1e-3);
        let mut by_index: BTreeMap<i64, Level> = BTreeMap::new();
        for t in &self.tris {
            if t.tilt_degrees() > max_tilt {
                continue;
            }
            let y = (t.v[0][1] + t.v[1][1] + t.v[2][1]) / 3.0;
            let idx = (y / bucket).floor() as i64;
            let e = by_index.entry(idx).or_insert(Level {
                y_lo: idx as f32 * bucket,
                y_hi: (idx as f32 + 1.0) * bucket,
                area_xz: 0.0,
                tris: 0,
                bmin: [f32::INFINITY; 2],
                bmax: [f32::NEG_INFINITY; 2],
            });
            e.area_xz += t.area_xz();
            e.tris += 1;
            for v in &t.v {
                e.bmin[0] = e.bmin[0].min(v[0]);
                e.bmin[1] = e.bmin[1].min(v[2]);
                e.bmax[0] = e.bmax[0].max(v[0]);
                e.bmax[1] = e.bmax[1].max(v[2]);
            }
        }
        by_index.into_values().collect()
    }

    /// How the box's surface area splits by tilt, in m² of XZ footprint.
    /// `buckets` are the upper tilt bounds in degrees.
    pub fn slope_profile(&self, buckets: &[f32]) -> Vec<(f32, f32)> {
        let mut out: Vec<(f32, f32)> = buckets.iter().map(|b| (*b, 0.0)).collect();
        for t in &self.tris {
            let tilt = t.tilt_degrees();
            if let Some(slot) = out.iter_mut().find(|(b, _)| tilt <= *b) {
                slot.1 += t.area_xz();
            }
        }
        out
    }

    /// Rasterise wall-like geometry in the height band `[y_lo, y_hi]` onto an
    /// XZ grid. A cell is blocked when any triangle steeper than `min_tilt`
    /// passes through it inside the band — that is, when something an agent
    /// cannot walk over stands in the way.
    pub fn occupancy(&self, y_lo: f32, y_hi: f32, cell: f32, min_tilt: f32) -> Occupancy {
        let w = (((self.bmax[0] - self.bmin[0]) / cell).ceil() as usize).max(1);
        let h = (((self.bmax[2] - self.bmin[2]) / cell).ceil() as usize).max(1);
        let mut blocked = vec![false; w * h];
        for t in &self.tris {
            if t.tilt_degrees() < min_tilt {
                continue;
            }
            let lo = t.bmin();
            let hi = t.bmax();
            if hi[1] < y_lo || lo[1] > y_hi {
                continue;
            }
            // Conservative: mark the triangle's XZ bounding box. At cell 0.1 m
            // a wall panel's bbox is the panel, and over-marking a diagonal
            // by a few centimetres never turns a real opening into a closed
            // one by more than the cell size.
            // A perfectly axis-aligned wall panel has zero extent on one
            // axis, and floor == ceil would rasterise it to nothing. Always
            // cover at least the one cell the panel sits in.
            let x0 = ((((lo[0] - self.bmin[0]) / cell).floor().max(0.0)) as usize).min(w - 1);
            let x1 =
                (((((hi[0] - self.bmin[0]) / cell).ceil()).max(0.0) as usize).max(x0 + 1)).min(w);
            let z0 = ((((lo[2] - self.bmin[2]) / cell).floor().max(0.0)) as usize).min(h - 1);
            let z1 =
                (((((hi[2] - self.bmin[2]) / cell).ceil()).max(0.0) as usize).max(z0 + 1)).min(h);
            for z in z0..z1 {
                for x in x0..x1 {
                    blocked[z * w + x] = true;
                }
            }
        }
        Occupancy {
            bmin: [self.bmin[0], self.bmin[2]],
            cell,
            w,
            h,
            blocked,
        }
    }
}

/// One height bucket of [`Slab::level_histogram`].
#[derive(Debug, Clone, Copy)]
pub struct Level {
    pub y_lo: f32,
    pub y_hi: f32,
    /// XZ footprint of the near-horizontal triangles in this bucket, m².
    pub area_xz: f32,
    pub tris: usize,
    /// XZ extent, `[x, z]`.
    pub bmin: [f32; 2],
    pub bmax: [f32; 2],
}

/// XZ occupancy grid over one slab's footprint.
#[derive(Debug, Clone)]
pub struct Occupancy {
    pub bmin: [f32; 2],
    pub cell: f32,
    pub w: usize,
    pub h: usize,
    pub blocked: Vec<bool>,
}

impl Occupancy {
    pub fn is_blocked(&self, x: f32, z: f32) -> bool {
        let ix = ((x - self.bmin[0]) / self.cell).floor();
        let iz = ((z - self.bmin[1]) / self.cell).floor();
        if ix < 0.0 || iz < 0.0 {
            return false;
        }
        let (ix, iz) = (ix as usize, iz as usize);
        if ix >= self.w || iz >= self.h {
            return false;
        }
        self.blocked[iz * self.w + ix]
    }

    /// Walk the segment `a → b` and return the unblocked runs along it, as
    /// `(start_metres, end_metres)` measured from `a`.
    ///
    /// This is the clear-width measurement: draw the line across a doorway
    /// and the widest run is the opening. A run narrower than
    /// `2 * agentRadius` will be eroded shut by Recast.
    pub fn free_runs(&self, a: [f32; 2], b: [f32; 2]) -> Vec<(f32, f32)> {
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        if len <= 0.0 {
            return Vec::new();
        }
        let steps = ((len / (self.cell * 0.5)).ceil() as usize).max(1);
        let mut runs = Vec::new();
        let mut start: Option<f32> = None;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let d = t * len;
            let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            if self.is_blocked(p[0], p[1]) {
                if let Some(s) = start.take() {
                    runs.push((s, d));
                }
            } else if start.is_none() {
                start = Some(d);
            }
        }
        if let Some(s) = start {
            runs.push((s, len));
        }
        runs
    }

    /// Width of the widest clear run along `a → b`.
    pub fn widest_free_run(&self, a: [f32; 2], b: [f32; 2]) -> f32 {
        self.free_runs(a, b)
            .into_iter()
            .map(|(s, e)| e - s)
            .fold(0.0f32, f32::max)
    }
}

/// A batch of boxes, filled in one pass over the chunk directory.
#[derive(Debug)]
pub struct SlabSet {
    pub slabs: Vec<Slab>,
    /// Extra metres a chunk's grid cell is grown by before deciding it cannot
    /// touch any box. Actors are placed in the chunk that owns them but their
    /// meshes overhang.
    pub margin: f32,
    /// Chunk stems actually opened.
    pub chunks_read: Vec<String>,
    pub chunks_skipped: usize,
}

impl SlabSet {
    pub fn new(slabs: Vec<Slab>) -> Self {
        Self {
            slabs,
            margin: 60.0,
            chunks_read: Vec::new(),
            chunks_skipped: 0,
        }
    }

    /// Read every `<hex8>o.obj` in `dir` that could touch a box, and fill the
    /// slabs.
    pub fn load(&mut self, dir: &Path) -> io::Result<()> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("obj"))
            .collect();
        files.sort();

        for path in files {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            if !self.chunk_may_touch(&stem) {
                self.chunks_skipped += 1;
                continue;
            }
            self.chunks_read.push(stem.clone());
            self.read_obj(&path, &stem)?;
        }
        Ok(())
    }

    /// Chunk-grid rejection test. Unparseable stems are always read — better
    /// slow than silently missing geometry.
    fn chunk_may_touch(&self, stem: &str) -> bool {
        let Some(hex) = stem.strip_suffix('o') else {
            return true;
        };
        let Ok(raw) = u32::from_str_radix(hex, 16) else {
            return true;
        };
        let id = ChunkId::from_raw(raw);
        // Low u16 → BW x index, high u16 → BW z index, 100 m per cell.
        let x0 = id.position_x() as f32 * 100.0 - self.margin;
        let x1 = (id.position_x() as f32 + 1.0) * 100.0 + self.margin;
        let z0 = id.position_z() as f32 * 100.0 - self.margin;
        let z1 = (id.position_z() as f32 + 1.0) * 100.0 + self.margin;
        self.slabs
            .iter()
            .any(|s| s.bmax[0] >= x0 && s.bmin[0] <= x1 && s.bmax[2] >= z0 && s.bmin[2] <= z1)
    }

    fn read_obj(&mut self, path: &Path, stem: &str) -> io::Result<()> {
        let file = std::fs::File::open(path)?;
        let mut reader = BufReader::with_capacity(1 << 20, file);
        let mut verts: Vec<[f32; 3]> = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            let s = line.trim_end();
            if let Some(rest) = s.strip_prefix("v ") {
                if let Some(v) = parse_vertex(rest) {
                    verts.push(v);
                }
            } else if let Some(rest) = s.strip_prefix("f ") {
                let Some(idx) = parse_face(rest, verts.len()) else {
                    continue;
                };
                let tri = Tri {
                    v: [verts[idx[0]], verts[idx[1]], verts[idx[2]]],
                };
                for slab in &mut self.slabs {
                    if tri.overlaps(slab.bmin, slab.bmax) {
                        slab.tris.push(tri);
                        *slab.by_chunk.entry(stem.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
        Ok(())
    }
}

/// `v <ue.x> <ue.z> <ue.y>` in centimetres → BigWorld metres.
fn parse_vertex(rest: &str) -> Option<[f32; 3]> {
    let mut it = rest.split_whitespace();
    let c0: f32 = it.next()?.parse().ok()?;
    let c1: f32 = it.next()?.parse().ok()?;
    let c2: f32 = it.next()?.parse().ok()?;
    // mesh.cpp:106-108 — bw = (col2, col1, col0) / 100.
    Some([c2 / 100.0, c1 / 100.0, c0 / 100.0])
}

/// `f a b c` with 1-based indices; `a/b/c` forms are tolerated. Returns
/// `None` for degenerate or out-of-range faces rather than panicking — a
/// truncated last line is a real thing in these files.
fn parse_face(rest: &str, nverts: usize) -> Option<[usize; 3]> {
    let mut out = [0usize; 3];
    let mut n = 0;
    for tok in rest.split_whitespace() {
        if n == 3 {
            return None; // quads are not emitted by the extractor
        }
        let first = tok.split('/').next()?;
        let i: usize = first.parse().ok()?;
        if i == 0 || i > nverts {
            return None;
        }
        out[n] = i - 1;
        n += 1;
    }
    if n == 3 {
        Some(out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;

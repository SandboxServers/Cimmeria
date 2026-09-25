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

mod spans;

use std::collections::HashSet;

use crate::format;
use crate::grid::{LayerKind, Occluder, TILE};
use crate::heightfield::HeightfieldAcc;
use crate::raster::{is_floor_like, Triangle};
use spans::{finish_layer, LayerAcc, Rec};

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
    /// With external coverage, the covered tiles of the geometry layer and
    /// the terrain heightfield, fixed at the first collision triangle:
    /// nothing outside them is rasterised.
    clip: Option<(HashSet<(i64, i64)>, HashSet<(i64, i64)>)>,
    /// Collision triangles that fell wholly outside the external coverage.
    trimmed: u64,
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
            // Coverage is marked in blocks no coarser than 4 m, then grown by
            // the margin; a wide margin must not coarsen the mask with it.
            block: params.margin.unwrap_or(0.0).clamp(1.0, 4.0),
            params,
            layers,
            floor_blocks: HashSet::new(),
            external_coverage: false,
            clip: None,
            trimmed: 0,
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

    /// Collision triangles dropped because they lie wholly outside the
    /// external coverage (a terrain patch or a whole mesh far from any
    /// explorable area). A triangle straddling the edge is clipped, and not
    /// counted.
    pub fn trimmed_count(&self) -> u64 {
        self.trimmed
    }

    /// Add one triangle, in BigWorld metres. Non-finite triangles are
    /// dropped.
    pub fn add_triangle(&mut self, tri: &Triangle, source: Source) {
        if !tri.iter().flatten().all(|v| v.is_finite()) {
            return;
        }
        self.triangles += 1;
        // Order-independent: the extractor's triangle order is not stable
        // from run to run (hash-map walks in the StaticMesh extraction), and
        // two builds of the same map must agree byte for byte.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ source as u64;
        for v in tri.iter().flatten() {
            for b in v.to_bits().to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        self.hash = self.hash.wrapping_add(h ^ (h >> 29));
        if self.params.margin.is_some()
            && !self.external_coverage
            && is_floor_like(tri, self.params.max_floor_slope_deg)
        {
            self.mark_floor(tri);
        }
        if self.external_coverage && self.params.margin.is_some() && self.clip.is_none() {
            let geo = self.covered_tiles(self.params.cell, std::iter::empty());
            let ter = self
                .params
                .terrain_pitch
                .map(|p| self.covered_tiles(p, std::iter::empty()))
                .unwrap_or_default();
            self.clip = Some((geo, ter));
        }
        if source == Source::Terrain {
            if let Some(hf) = &mut self.terrain {
                if let Some((_, ter)) = &self.clip {
                    if !touches_tiles(tri, hf.pitch() * TILE as f32, ter) {
                        self.trimmed += 1;
                        return;
                    }
                }
                if hf.add(tri) {
                    return;
                }
                self.terrain_fallback += 1;
            }
        }
        if let Some((geo, _)) = &self.clip {
            if !touches_tiles(tri, self.params.cell * TILE as f32, geo) {
                self.trimmed += 1;
                return;
            }
        }
        let allowed = self.clip.as_ref().map(|(g, _)| g);
        self.layers[0].rasterise(tri, self.params.y_step, allowed);
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
        // Coverage added after geometry would not clip what came before.
        debug_assert!(self.clip.is_none(), "add coverage before triangles");
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

/// Whether the XZ bounding box of `tri` meets any tile (edge `tile_w`) of
/// `tiles`.
fn touches_tiles(tri: &Triangle, tile_w: f32, tiles: &HashSet<(i64, i64)>) -> bool {
    let (mut x0, mut x1, mut z0, mut z1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for v in tri {
        x0 = x0.min(v[0]);
        x1 = x1.max(v[0]);
        z0 = z0.min(v[2]);
        z1 = z1.max(v[2]);
    }
    let (ax, bx) = ((x0 / tile_w).floor() as i64, (x1 / tile_w).floor() as i64);
    let (az, bz) = ((z0 / tile_w).floor() as i64, (z1 / tile_w).floor() as i64);
    if ((bx - ax + 1) * (bz - az + 1)) as usize > tiles.len() {
        return tiles
            .iter()
            .any(|&(tx, tz)| (ax..=bx).contains(&tx) && (az..=bz).contains(&tz));
    }
    (az..=bz).any(|tz| (ax..=bx).any(|tx| tiles.contains(&(tx, tz))))
}

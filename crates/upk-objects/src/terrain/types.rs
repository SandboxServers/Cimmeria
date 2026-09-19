//! Decoded `Terrain` actor: grid dimensions, placement, heightmap and
//! per-vertex info flags.

/// UE3's fixed height quantisation: a raw `u16` height is
/// `(h - 32768) * TERRAIN_ZSCALE` in the terrain actor's **local**
/// space, before `DrawScale`/`DrawScale3D` are applied.
///
/// Confirmed against real data: every flat SGW terrain stores `0x8000`
/// for all vertices, which this formula maps to local Z = 0 exactly.
/// The shipped `data/spaces/castle_cellblock.nav` places its 637,283 m²
/// ground sheet at BW y ≈ 0.2 while all 1600 Castle_CellBlock terrain
/// actors sit at `Location.Z = 0` — only a `(h - 32768)` bias reproduces
/// that. (An earlier `h / 65535 * 256` guess would have put it at
/// BW y 1.28.)
pub const TERRAIN_ZSCALE: f32 = 1.0 / 128.0;

/// The raw height value that means "no displacement".
pub const TERRAIN_NEUTRAL_HEIGHT: u16 = 0x8000;

/// `TID_Visibility_Off` — bit 0 of an `InfoData` byte marks the quad
/// whose **lower-left** corner is that vertex as a hole.
///
/// Every `InfoData` byte in the SGW Castle / Castle_CellBlock /
/// Harset / Agnos / SGC maps is either `0x00` or `0x01`, so no other
/// bit is in use in shipped content.
pub const TID_VISIBILITY_OFF: u8 = 0x01;

/// SGW's `ATerrain` class default for `DrawScale3D`.
///
/// **Not** the generic UE3 actor default of `(1, 1, 1)`. Terrain actors
/// omit the property when it matches the class default, and the cooked
/// data only makes sense at `(100, 100, 100)`:
///
/// - `Castle_CellBlock` places 1600 terrain actors of 20 patches each on
///   an exact 2000 cm grid ⇒ 100 cm per patch.
/// - `Castle` places 144 terrain actors of 100 patches each on an exact
///   10000 cm grid ⇒ 100 cm per patch. All 144 omit `DrawScale3D`.
/// - The Z component is pinned by the outdoor gate-room/DHD seed point
///   (BW y = 55.10): the decoded Castle terrain under it is BW y 55.14
///   with Z = 100, and 110.28 with Z = 200.
///
/// 1200 of the 1600 `Castle_CellBlock` actors *do* write `DrawScale3D`
/// explicitly — as `(100, 100, 200)`. That is consistent: UE3 serialises
/// a property only when it differs from the default, and `(100,100,200)`
/// differs from `(100,100,100)` in Z. Those actors are all flat
/// (`0x8000` everywhere) so their doubled Z scale is unobservable.
pub const SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D: [f32; 3] = [100.0, 100.0, 100.0];

/// A decoded UE3 `Terrain` actor export.
///
/// Layers, weight maps and the lighting/foliage trailer are consumed but
/// not modelled — they carry no collision information. The number of
/// bytes left undecoded is recorded in
/// [`Terrain::lighting_trailer_bytes`] so callers (and tests) can pin
/// the exact consumption against ground truth.
#[derive(Debug, Clone)]
pub struct Terrain {
    /// Quad counts along each axis.
    pub num_patches_x: u32,
    pub num_patches_y: u32,
    /// Heightmap dimensions — always `num_patches_* + 1`.
    pub num_vertices_x: u32,
    pub num_vertices_y: u32,
    /// Render/LOD partitioning of this one actor into
    /// `num_sections_x * num_sections_y` `TerrainComponent` exports.
    /// Irrelevant to collision: the heightmap below already covers the
    /// whole actor. Recorded because it explains why a Castle chunk has
    /// 25 `TerrainComponent` exports but only one `Terrain`.
    pub num_sections_x: u32,
    pub num_sections_y: u32,
    /// Render tessellation ceiling. Never present in shipped SGW
    /// content (defaults to 1); collision uses the base patch grid
    /// regardless.
    pub max_tesselation_level: u32,

    /// Actor placement, in **world** UE3 cm — terrain `Location` is
    /// absolute, not chunk-relative.
    pub location: [f32; 3],
    /// Raw UE3 rotator `(pitch, yaw, roll)`.
    pub rotation: [i32; 3],
    pub draw_scale: f32,
    /// Defaults to [`SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D`] when the
    /// property is absent, not to `(1, 1, 1)`.
    pub draw_scale_3d: [f32; 3],

    /// `num_vertices_x * num_vertices_y` raw heights, row-major with X
    /// varying fastest.
    pub heights: Vec<u16>,
    /// Same length as [`Terrain::heights`]; bit 0 is
    /// [`TID_VISIBILITY_OFF`].
    pub info_data: Vec<u8>,

    /// Alpha-map texel dimensions, from the binary copies in the
    /// trailer (cross-checked against the tagged-property values).
    pub alpha_x_size: u32,
    pub alpha_y_size: u32,
    /// Counts consumed from the trailer; the payloads are skipped.
    pub weighted_texture_map_count: u32,
    pub weight_map_texture_count: u32,

    /// Bytes after `WeightMapTextures.Num` that this decoder does not
    /// model — lighting GUIDs and foliage proxy data. Observed values:
    /// 152 (Castle_CellBlock), 164–3304 (Castle, Harset, Agnos),
    /// 92 (SGC). Always >= 0; a value that would be negative is
    /// reported as a parse error instead.
    pub lighting_trailer_bytes: usize,
}

impl Terrain {
    /// Number of quads in the collision grid.
    pub fn quad_count(&self) -> usize {
        self.num_patches_x as usize * self.num_patches_y as usize
    }

    /// Raw height at grid vertex `(i, j)`.
    ///
    /// Returns `None` when either index is outside the heightmap.
    pub fn raw_height(&self, i: u32, j: u32) -> Option<u16> {
        if i >= self.num_vertices_x || j >= self.num_vertices_y {
            return None;
        }
        self.heights
            .get(j as usize * self.num_vertices_x as usize + i as usize)
            .copied()
    }

    /// The **actor-local** position of grid vertex `(i, j)`, in the
    /// pre-`DrawScale` space UE3 uses: one unit per patch in X/Y, and
    /// `(h - 32768) * TERRAIN_ZSCALE` in Z.
    ///
    /// Feed this straight into the navmesh extractor's `ActorTransform`
    /// — the scale/rotate/translate it applies is exactly UE3's
    /// terrain LocalToWorld.
    pub fn local_vertex(&self, i: u32, j: u32) -> Option<[f32; 3]> {
        let h = self.raw_height(i, j)?;
        Some([
            i as f32,
            j as f32,
            (h as f32 - TERRAIN_NEUTRAL_HEIGHT as f32) * TERRAIN_ZSCALE,
        ])
    }

    /// Whether the quad with lower-left corner `(i, j)` is rendered and
    /// collidable.
    ///
    /// UE3 stores visibility per-vertex but reads it per-quad, indexed
    /// by the quad's lower-left corner (`ATerrain::IsTerrainQuadVisible`)
    /// — so the final heightmap row and column never gate a quad. An
    /// out-of-range index is reported as invisible so a malformed grid
    /// can't emit geometry.
    pub fn quad_visible(&self, i: u32, j: u32) -> bool {
        if i >= self.num_patches_x || j >= self.num_patches_y {
            return false;
        }
        let idx = j as usize * self.num_vertices_x as usize + i as usize;
        match self.info_data.get(idx) {
            Some(b) => b & TID_VISIBILITY_OFF == 0,
            None => false,
        }
    }

    /// Count of quads suppressed by [`TID_VISIBILITY_OFF`].
    pub fn hole_quad_count(&self) -> usize {
        let mut n = 0;
        for j in 0..self.num_patches_y {
            for i in 0..self.num_patches_x {
                if !self.quad_visible(i, j) {
                    n += 1;
                }
            }
        }
        n
    }
}

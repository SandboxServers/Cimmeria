//! `Terrain` export bodies.
//!
//! Encodes what `cimmeria_upk_objects::deserialize_terrain` reads: a
//! 32-byte `AActor` header, the tagged properties it looks up, and the
//! native trailer `ATerrain::Serialize` writes:
//!
//! ```text
//! i32  Heights.Num   (== NumVerticesX * NumVerticesY)
//! u16  Heights[..]
//! i32  InfoData.Num  (== Heights.Num)
//! u8   InfoData[..]  (bit 0 = TID_Visibility_Off)
//! i32  AlphaXSize    (cross-checked against the tagged property)
//! i32  AlphaYSize
//! i32  WeightedTextureMaps.Num
//! i32  WeightMapTextures.Num
//! ```
//!
//! The decoder enforces the `NumVertices == NumPatches + 1` invariant
//! and both alpha cross-checks, so a fixture that gets the grid or the
//! alpha sizes wrong fails at decode rather than producing plausible
//! geometry.

use cimmeria_upk_objects::terrain::TERRAIN_NEUTRAL_HEIGHT;
use cimmeria_upk_objects::SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D;

use super::package_bytes::PackageBuilder;

/// Number of zero bytes before an `AActor`'s property stream.
const ACTOR_HEADER: usize = 32;

/// A synthetic terrain grid.
#[derive(Debug, Clone)]
pub struct TerrainPayload {
    pub patches_x: u32,
    pub patches_y: u32,
    pub location: [f32; 3],
    pub rotation: [i32; 3],
    pub draw_scale: f32,
    pub draw_scale_3d: [f32; 3],
    /// `(patches_x + 1) * (patches_y + 1)` raw heights, row-major in x.
    pub heights: Vec<u16>,
    /// Same length; bit 0 set marks the vertex `TID_Visibility_Off`,
    /// which holes the quad whose lower-left corner it is.
    pub info: Vec<u8>,
}

impl TerrainPayload {
    /// A flat, hole-free grid at the terrain neutral height, placed at
    /// the origin with the SGW class-default `DrawScale3D` (one patch
    /// = 100 cm).
    pub fn flat(patches_x: u32, patches_y: u32) -> Self {
        let n = ((patches_x + 1) * (patches_y + 1)) as usize;
        Self {
            patches_x,
            patches_y,
            location: [0.0; 3],
            rotation: [0; 3],
            draw_scale: 1.0,
            draw_scale_3d: SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D,
            heights: vec![TERRAIN_NEUTRAL_HEIGHT; n],
            info: vec![0; n],
        }
    }

    /// Place the terrain in absolute world space.
    pub fn at(mut self, location: [f32; 3]) -> Self {
        self.location = location;
        self
    }

    /// Hole the quad whose lower-left grid vertex is `(i, j)`.
    pub fn with_hole(mut self, i: u32, j: u32) -> Self {
        let idx = self.vertex_index(i, j);
        self.info[idx] = 1;
        self
    }

    /// Raise grid vertex `(i, j)` by `units` raw height steps (128 raw
    /// = 1 local unit = 100 cm at the default Z scale).
    pub fn raise(mut self, i: u32, j: u32, units: i32) -> Self {
        let idx = self.vertex_index(i, j);
        self.heights[idx] = (i32::from(TERRAIN_NEUTRAL_HEIGHT) + units) as u16;
        self
    }

    fn vertex_index(&self, i: u32, j: u32) -> usize {
        assert!(
            i <= self.patches_x && j <= self.patches_y,
            "vertex off grid"
        );
        (j * (self.patches_x + 1) + i) as usize
    }

    /// Encode the export body, interning property names into `pkg`.
    pub fn encode(&self, pkg: &mut PackageBuilder) -> Vec<u8> {
        let expected = ((self.patches_x + 1) * (self.patches_y + 1)) as usize;
        assert_eq!(self.heights.len(), expected, "heights must cover the grid");
        assert_eq!(self.info.len(), expected, "info must cover the grid");

        let mut body = vec![0u8; ACTOR_HEADER];
        let mut props = pkg.props();
        props
            .int("NumPatchesX", self.patches_x as i32)
            .int("NumPatchesY", self.patches_y as i32)
            .int("NumVerticesX", (self.patches_x + 1) as i32)
            .int("NumVerticesY", (self.patches_y + 1) as i32)
            .int("NumSectionsX", 1)
            .int("NumSectionsY", 1)
            .int("MaxTesselationLevel", 1)
            .placement(
                self.location,
                self.rotation,
                self.draw_scale,
                self.draw_scale_3d,
            )
            .int("AlphaXSize", 0)
            .int("AlphaYSize", 0);
        body.extend_from_slice(&props.finish());

        body.extend_from_slice(&(expected as i32).to_le_bytes());
        for h in &self.heights {
            body.extend_from_slice(&h.to_le_bytes());
        }
        body.extend_from_slice(&(expected as i32).to_le_bytes());
        body.extend_from_slice(&self.info);
        body.extend_from_slice(&0i32.to_le_bytes()); // AlphaXSize
        body.extend_from_slice(&0i32.to_le_bytes()); // AlphaYSize
        body.extend_from_slice(&0i32.to_le_bytes()); // WeightedTextureMaps.Num
        body.extend_from_slice(&0i32.to_le_bytes()); // WeightMapTextures.Num
        body
    }
}

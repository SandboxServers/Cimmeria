//! `Terrain` actor deserializer for UE3 export data (SGW v486).
//!
//! A `Terrain` export is a 32-byte `AActor` header, a UE3 tagged-property
//! stream terminated by the `None` FName, and then a native binary
//! trailer written by `ATerrain::Serialize`:
//!
//! ```text
//! +0x000  INT32   Heights.Num              (= NumVerticesX * NumVerticesY)
//! +0x004  UINT16  Heights[0..N-1]
//! +????   INT32   InfoData.Num             (= same N)
//! +????   UINT8   InfoData[0..N-1]         (bit 0 = TID_Visibility_Off)
//! +????   INT32   AlphaXSize               (binary copy of the property)
//! +????   INT32   AlphaYSize               (binary copy of the property)
//! +????   INT32   WeightedTextureMaps.Num
//! +????   [INT32 len + len bytes] * that count
//! +????   INT32   WeightMapTextures.Num
//!         --- everything past here is lighting GUIDs + foliage proxy
//!             data; counted, not decoded ---
//! ```
//!
//! Recovered by RE of `ATerrain::Serialize` @ `0x007517C0` in `SGW.exe`
//! (see `.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`)
//! and validated byte-exactly against 1600 `Castle_CellBlock` exports,
//! 144 `Castle` exports, and spot checks in Harset / Agnos / SGC.
//!
//! # Two authoring conventions, one decoder
//!
//! SGW ships terrain in two shapes and a caller must handle both by
//! walking **every** `Terrain`-class export in a package:
//!
//! - `Castle`: one `Terrain` actor per 100 m chunk, 100×100 patches,
//!   partitioned into `NumSectionsX * NumSectionsY = 25`
//!   `TerrainComponent` exports (render/LOD partitions of one
//!   heightmap — not separate terrains).
//! - `Castle_CellBlock`: 25 separate 20×20-patch `Terrain` actors per
//!   chunk laid out on a 2000 cm grid, `NumSections = 1×1`.
//!
//! Both work out to 100 cm per patch. See
//! [`SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D`] for why that matters.
//!
//! # Property-skip discipline
//!
//! The tagged-property walk must be a **flat byte skip** — jump by each
//! tag's declared `size`. `Layers` and `TerrainComponents` are
//! `ArrayProperty` blobs whose `size` covers their entire nested
//! content, including the inner `None` terminators of their per-element
//! sub-streams. A parser that recurses into them trips over the first
//! inner `None` and mistakes it for the outer one.
//! `cimmeria_upk::parse_tagged_properties_with_end` already does the
//! flat skip, so we use it directly.

mod parse;
mod types;

pub use parse::deserialize_terrain;
pub use types::{
    Terrain, SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D, TERRAIN_NEUTRAL_HEIGHT, TERRAIN_ZSCALE,
    TID_VISIBILITY_OFF,
};

//! `UModel` / `UPolys` deserializers for UE3 export data (SGW Epic ver 486).
//!
//! A cooked SGW `.umap` carries BSP world geometry in two places:
//!
//! - the **persistent level's own `Model`** — the compiled CSG world
//!   geometry (floors, walls, ceilings), already in world space;
//! - one small **`Model` per placed `Brush`/`*Volume` actor** — that
//!   actor's standalone convex shape, in actor-local space.
//!
//! Both are the same `UModel` wire format. Cooked QA packages do **not**
//! strip `UPolys`, so the original un-split CSG polygons are recoverable
//! too; this module parses both, but the `Nodes`-based path is the one
//! the navmesh extractor consumes (it needs no polygon re-clipping).
//!
//! Wire layout after the export's 4-byte NetIndex prefix and tagged
//! property stream (`None`-terminated, empty for every sampled `Model`):
//!
//! ```text
//!  Bounds          FBoxSphereBounds     28B raw (7 x f32)
//!  Vectors         TArray<FVector>      4B count + 12B/elem
//!  Points          TArray<FVector>      4B count + 12B/elem
//!  Nodes           TArray<FBspNode>     4B count + 68B/elem
//!  (ArVer > 0x140) objref               4B                 -- unidentified
//!  Surfs           TArray<FBspSurf>     4B count + 56B/elem (ArVer > 0x1a0)
//!  Verts           TArray<FVert>        4B count + 24B/elem
//!  NumSharedSides  INT                  4B
//!  NumZones        INT                  4B  -- count into a fixed Zones[64]
//!  Zones[NumZones] FZoneProperties      24B/elem
//!  Polys           objref               4B
//!  LeafHulls       TArray<INT>          4B count + 4B/elem
//!  Leaves          TArray<FLeaf>        4B count + 4B/elem
//!  RootOutside     UBOOL                4B
//!  Linked          UBOOL                4B
//!  PortalNodes     TArray<INT>          4B count + 4B/elem
//!  (unidentified)  TArray<16B elem>     4B count + 16B/elem
//!  (ArVer >= 0x14d) INT                 4B  -- "NumUniqueVertices"(?)
//!  (unidentified)  TArray<40B elem>     4B count + 40B/elem
//! ```
//!
//! An empty `Model` is therefore exactly 108 bytes on the wire
//! (4 NetIndex + 8 `None` + 28 Bounds + 17 x 4B scalar/count fields),
//! which matches every stub `Model` export observed in
//! `Castle-000a0002.umap` — a useful arithmetic self-check on the field
//! order above.
//!
//! `UPolys` is a 4-byte NetIndex + tagged properties + a **three**-i32
//! header (`Count`, a discarded legacy `Max`, a discarded legacy objref)
//! followed by `Count` variable-length `FPoly` records of
//! `88 + 12 * NumVertices` bytes each.
//!
//! Both deserializers **require** that the declared fields consume the
//! export's serial data exactly; a short read or a trailing remainder is
//! an error, never a silent truncation. See
//! `docs/reverse-engineering/findings/bsp-model-polys-serialize.md` for
//! the Ghidra evidence trail behind each field.
//!
//! The module is split along the same seam as [`crate::static_mesh`]:
//! - [`types`] — decoded structs plus the node-fan triangulator the
//!   navmesh extractor consumes.
//! - [`parse`] — the byte-level deserializer and its size constants.

mod parse;
mod types;

pub use parse::{deserialize_model, deserialize_polys, FBSP_NODE_SIZE, FBSP_SURF_SIZE, FVERT_SIZE};
pub use types::{
    BspNode, BspSurf, BspTriangulation, BspVert, CollisionFilter, Model, ModelBounds, Poly, Polys,
    NF_NOT_CSG, NF_NOT_VIS_BLOCKING, NF_SHOOT_THROUGH, NON_COLLIDING_NODE_FLAGS,
    NON_COLLIDING_POLY_FLAGS, PF_INVISIBLE, PF_NOT_SOLID, PF_PORTAL, PF_SEMISOLID, PF_TWO_SIDED,
};

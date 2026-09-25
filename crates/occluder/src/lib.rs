//! Per-space collision-geometry occluder for server-side line of sight
//! (NPC AI restoration NA27, issue #784, decision D-NA13).
//!
//! The navmesh raycast cannot see over a desk or under a ceiling: Recast
//! cuts every obstacle out of the walkable surface as a hole with no height,
//! so 45% of its same-storey `Blocked` answers on Castle_CellBlock were
//! false (NA16). An occluder answers the question from the collision
//! geometry itself: a column grid over the world's XZ plane where each cell
//! lists the solid Y spans above it, built at extraction time from the same
//! triangles NavBuilder consumes and shipped as `data/spaces/<world>.occ`.
//!
//! - [`build`]: [`OccluderBuilder`] rasterises triangles into the grid.
//! - [`format`]: the `.occ` file (header, then zlib per layer).
//! - [`query`]: [`Occluder::sight`], the eye-to-eye segment test.
//!
//! Coordinates are BigWorld metres throughout: X east, **Y up**, Z north.
//! The extractor converts from UE3 centimetres
//! (`bw = (ue.y, ue.z, ue.x) / 100`) before a triangle reaches this crate.

pub mod build;
pub mod format;
mod grid;
mod heightfield;
pub mod paged;
mod query;
pub mod raster;

pub use build::{BuildError, BuildParams, OccluderBuilder, Source};
pub use format::{OccluderError, MAGIC, VERSION};
pub use grid::{Layer, LayerKind, Occluder, TILE};
pub use heightfield::Heightfield;
pub use paged::{encode_paged, PageStats, PagedOccluder, Residency, DEFAULT_PAGE_SIZE};
pub use query::Sight;
pub use raster::Triangle;

#[cfg(test)]
mod tests;

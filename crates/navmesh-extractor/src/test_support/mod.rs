//! Synthetic UE3 package fixtures — enough of a cooked `.umap` to drive
//! the real walkers without the proprietary client tree.
//!
//! # Why this exists
//!
//! Every test that exercises [`crate::extract_map_with_report`], the
//! BSP walker, the terrain walker or the `extract_map` binary needs a
//! `cimmeria_upk::Package`. The cooked `CookedPC` tree can never be
//! committed, so those tests self-skip in CI and the code they cover
//! reports 0 %. The fix is not to un-skip them: it is to hand the same
//! walkers a package we build ourselves.
//!
//! # Why it lives here and not in `cimmeria-upk`
//!
//! [`cimmeria_upk::Package`]'s `decompressed` / `filepath` fields are
//! private, so a `Package` cannot be constructed in memory from outside
//! that crate — and `extract_map` takes a *directory of chunk files*
//! anyway, so at least one fixture has to reach the disk. That leaves a
//! byte-level writer, and a byte-level writer for a `.umap` has to
//! encode `Terrain`, `Model` and `StaticMesh` payloads, whose layout
//! constants live in `cimmeria-upk-objects` — the layer *above*
//! `cimmeria-upk`. This crate is the only one that already depends on
//! both, so it is the only place the whole builder can live without
//! either duplicating layout constants or inverting the dependency.
//!
//! Gated behind `#[cfg(any(test, feature = "test-support"))]`, mirroring
//! `cimmeria_mercury::test_harness`, so nothing here reaches a release
//! binary. `crates/navmesh-extractor/Cargo.toml` dev-depends on this
//! crate with the feature on, which is what makes the module visible to
//! the integration tests under `tests/`.
//!
//! # Round-trip contract
//!
//! Everything [`PackageBuilder`] emits is an **uncompressed** Epic-486
//! package that goes back in through the real [`cimmeria_upk::Package::open`]
//! — header, name table, import table, export table, export bodies. No
//! parser is stubbed or bypassed. `test_support::tests` pins that
//! round-trip; if the reader's field order ever changes, these fixtures
//! break first.
//!
//! # Layout
//!
//! - [`names`] — the shared name-table interner.
//! - [`props`] — UE3 tagged-property stream builder.
//! - [`package_bytes`] — the package writer itself.
//! - [`terrain_payload`], [`model_payload`], [`static_mesh_payload`] —
//!   per-class export bodies.
//! - [`chunk_fixtures`] — composes the above into whole `.umap` chunks
//!   and a matching [`cimmeria_upk_objects::PackageIndex`].

pub mod chunk_fixtures;
pub mod model_payload;
pub mod names;
pub mod package_bytes;
pub mod props;
pub mod static_mesh_payload;
pub mod terrain_payload;

pub use chunk_fixtures::{
    index_over, mesh_package, scratch_dir, ChunkFixture, ACTOR_PROPS_OFFSET, COMPONENT_PROPS_OFFSET,
};
pub use model_payload::{ModelPayload, NodeSpec, SurfSpec};
pub use names::NameTable;
pub use package_bytes::PackageBuilder;
pub use props::PropStream;
pub use static_mesh_payload::StaticMeshPayload;
pub use terrain_payload::TerrainPayload;

#[cfg(test)]
mod tests;

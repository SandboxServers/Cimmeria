//! Navmesh extractor — converts UE3 `.umap` chunk packages into OBJ
//! collision geometry for the C++ NavBuilder Recast pipeline.
//!
//! # Architecture
//!
//! ```text
//! crates/upk + crates/upk-objects   ──►   this crate   ──►   *.obj files
//!                                                                  │
//!                                                                  ▼
//!                                              deprecated/cpp/src/nav_builder
//!                                                                  │
//!                                                                  ▼
//!                                                          data/spaces/*.nav
//!                                                                  │
//!                                                                  ▼
//!                                          crates/entity/src/navigation.rs (runtime)
//! ```
//!
//! We deliberately **do not** port NavBuilder to Rust at this phase — the
//! Recast configuration tuning is the cheap part of the pipeline, and the
//! existing C++ build emits a `.nav` byte-format already validated against
//! `NavMesh::load`. Geometry extraction lives in Rust where the
//! `crates/upk*` stack already knows how to parse UE3 packages.
//!
//! # Phase status
//!
//! This crate currently ships **Phase 0 (.nav round-trip smoke)** and the
//! **Phase 1.1 scaffolding** (module skeleton + chunk-position decoding).
//! Phase 1.2 (StaticMesh extraction) and Phase 1.3 (Terrain decode) land
//! in follow-up changes — their module hooks are wired into [`extract_map`]
//! as `// TODO:` markers.

pub mod chunk_id;
pub mod geometry;
pub mod nav_roundtrip;
pub mod obj;
pub mod umap;

use std::path::Path;

/// Errors produced by the extractor.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UPK parse error: {0}")]
    Upk(#[from] cimmeria_upk::UpkError),

    #[error("Invalid chunk filename: {0}")]
    InvalidChunkFilename(String),

    #[error("Round-trip mismatch at offset {offset}: original=0x{original:02x} re-emitted=0x{reemitted:02x}")]
    RoundTripMismatch {
        offset: usize,
        original: u8,
        reemitted: u8,
    },

    #[error("Round-trip size mismatch: original={original} bytes, re-emitted={reemitted} bytes")]
    RoundTripSizeMismatch { original: usize, reemitted: usize },

    /// A count field in a `.nav` header is implausibly large — either an
    /// arithmetic overflow when computing the allocation size, or a value
    /// that exceeds the documented `MAX_*` caps. Castle Cellblock — the
    /// largest real navmesh in the SGW data set at the time of writing —
    /// has `nverts=2778, npolys=1479`, so the caps in `nav_roundtrip.rs`
    /// (1M each) sit four orders of magnitude above ground truth.
    /// Triggered exclusively by malformed or hostile input.
    #[error("Implausible {field} = {value} in .nav header ({reason})")]
    NavHeaderOutOfRange {
        field: &'static str,
        value: u64,
        reason: &'static str,
    },

    #[error("Other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ExtractError>;

/// Orchestrator entry point: extract collision geometry from every `.umap`
/// chunk in `map_dir` and write per-chunk `.obj` files into `output_dir`.
///
/// The output filename convention matches what the C++ NavBuilder
/// expects: `<XXXXYYYY>o.obj`, where the eight hex digits before the `o`
/// are the chunk ID as stored in the UE3 cooked filename
/// (e.g. `Castle_CellBlock-FFFEFFFD.umap` → `fffefffdo.obj`). NavBuilder
/// decodes those eight digits back into a `(positionX, positionZ)` pair
/// via [`chunk_id::ChunkId::decode`]; see that module for axis-label
/// caveats.
///
/// # Phase status
///
/// Currently a **stub** — it walks the map directory and prints the
/// chunk IDs but does not yet emit geometry. The StaticMesh decoder
/// (Phase 1.2) and Terrain decoder (Phase 1.3) plug into the
/// `// TODO:` markers inside the per-chunk loop.
pub fn extract_map(map_dir: &Path, output_dir: &Path) -> Result<()> {
    tracing::info!(map_dir = %map_dir.display(), output_dir = %output_dir.display(), "extract_map: starting");

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    let chunks = umap::enumerate_chunks(map_dir)?;
    tracing::info!(count = chunks.len(), "extract_map: enumerated chunks");

    for chunk_path in chunks {
        let id = chunk_id::ChunkId::from_umap_path(&chunk_path)?;
        tracing::debug!(
            chunk = %chunk_path.display(),
            chunk_id = format!("{:08x}", id.raw()),
            position_x = id.position_x(),
            position_z = id.position_z(),
            "extract_map: processing chunk"
        );

        // Phase 1.2: StaticMesh extraction goes here. Open the umap with
        // `cimmeria_upk::Package`, walk StaticMeshActors, resolve
        // StaticMeshComponent → StaticMesh via PackageIndex, transform
        // LOD0 vertices into world space, push triangles into a
        // `geometry::TriangleSoup`.

        // Phase 1.3: Terrain extraction goes here. For each `Terrain`
        // export, parse the tagged-property block, then decode the
        // binary trailer (Heights → InfoData → AlphaXSize → AlphaYSize
        // → WeightedTextureMaps → WeightMapTextures), triangulate via
        // `geometry::triangulate_terrain`. The recipe is documented in
        // `.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`.

        // Phase 1.4: BSP Model/Polys — deferred; needs Ghidra trace.
    }

    Ok(())
}

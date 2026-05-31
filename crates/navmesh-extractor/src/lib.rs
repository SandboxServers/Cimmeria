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
//! This crate currently ships **Phase 0 (.nav round-trip smoke)**, the
//! **Phase 1.1 scaffolding** (module skeleton + chunk-position decoding),
//! and **Phase 1.2 (StaticMesh + StaticMeshActor extraction)**. Phase
//! 1.3 (Terrain decode) lands in a follow-up change — its module hook
//! is wired into [`extract_map`] as a `// TODO:` marker.

pub mod chunk_id;
pub mod geometry;
pub mod nav_roundtrip;
pub mod obj;
pub mod staticmesh;
pub mod transform;
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
/// via [`chunk_id::ChunkId::from_obj_path`]; see that module for
/// axis-label caveats.
///
/// `index` is the cross-package export index used to resolve
/// `StaticMeshActor` → `StaticMesh` references. Build it once via
/// [`cimmeria_upk_objects::PackageIndex::build`] from the `CookedPC`
/// directory and reuse it across maps. Passing `None` runs in degraded
/// mode — actors are still walked and logged, but no triangles are
/// emitted; useful for CI runs that don't ship the cooked asset bundle.
///
/// # Phase status
///
/// Ships **Phase 1.2 (StaticMesh extraction)**. Phase 1.3 (Terrain) is
/// still a `// TODO:` marker inside the per-chunk loop.
pub fn extract_map(
    map_dir: &Path,
    output_dir: &Path,
    index: Option<&cimmeria_upk_objects::PackageIndex>,
) -> Result<()> {
    tracing::info!(map_dir = %map_dir.display(), output_dir = %output_dir.display(), "extract_map: starting");

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    let chunks = umap::enumerate_chunks(map_dir)?;
    tracing::info!(count = chunks.len(), "extract_map: enumerated chunks");

    // Combined OBJ accumulator — emitted alongside the per-chunk files
    // so the NavBuilder operator can pick `whole` mode if they want to
    // build a single navmesh from the entire map without chunk
    // boundaries. The filename `<mapname>.obj` mirrors what the legacy
    // C++ extractor produced.
    let mut combined_soups: Vec<geometry::TriangleSoup> = Vec::new();

    let mut chunks_with_geometry = 0usize;
    let mut total_triangles = 0usize;
    let mut total_actors_resolved = 0usize;
    let mut total_actors_unresolved = 0usize;

    for chunk_path in chunks {
        let id = chunk_id::ChunkId::from_umap_path(&chunk_path)?;
        tracing::debug!(
            chunk = %chunk_path.display(),
            chunk_id = format!("{:08x}", id.raw()),
            position_x = id.position_x(),
            position_z = id.position_z(),
            "extract_map: processing chunk"
        );

        // Phase 1.2: StaticMesh extraction.
        let mut extraction = staticmesh::extract_chunk(&chunk_path, index)?;
        // Tag the soup with a group so NavBuilder can debug-print which
        // chunk a triangle came from. `Chunk_*` keeps it distinct from
        // the reserved `Terrain_*` prefix NavBuilder skips.
        extraction.soup.group = Some(format!("Chunk_{:08x}", id.raw()));

        total_actors_resolved += extraction.actors_resolved;
        total_actors_unresolved += extraction.actors_unresolved;
        total_triangles += extraction.triangles_emitted;

        // Phase 1.3: Terrain extraction. For each `Terrain` export,
        // parse the tagged-property block, then decode the binary
        // trailer (Heights → InfoData → AlphaXSize → AlphaYSize →
        // WeightedTextureMaps → WeightMapTextures), triangulate via
        // `geometry::triangulate_terrain`. The recipe is documented in
        // `.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`.
        // TODO: wire `Terrain` exports into `extraction.soup` (Phase 1.3).

        // Phase 1.4: BSP Model/Polys — deferred; needs Ghidra trace.

        if extraction.soup.triangle_count() == 0 {
            // Empty chunks still get a stub OBJ so NavBuilder's chunked
            // mode sees the expected file set. Skip the write when no
            // geometry was extracted at all to avoid littering the output
            // dir with zero-content files.
            tracing::debug!(
                chunk_id = format!("{:08x}", id.raw()),
                actors_total = extraction.actors_total,
                actors_resolved = extraction.actors_resolved,
                actors_unresolved = extraction.actors_unresolved,
                "extract_map: chunk produced no geometry; skipping OBJ write"
            );
            continue;
        }

        let obj_path = output_dir.join(id.obj_filename());
        obj::write_obj(&obj_path, &extraction.soup)?;
        chunks_with_geometry += 1;

        tracing::info!(
            chunk_id = format!("{:08x}", id.raw()),
            actors_total = extraction.actors_total,
            actors_resolved = extraction.actors_resolved,
            triangles = extraction.triangles_emitted,
            path = %obj_path.display(),
            "extract_map: wrote chunk OBJ"
        );

        combined_soups.push(extraction.soup);
    }

    if !combined_soups.is_empty() {
        // Combined OBJ is named after the map directory (lowercase) so a
        // CI run on `Castle_CellBlock/` produces `castle_cellblock.obj`,
        // matching the historical naming convention from
        // `data/spaces/*.nav`.
        let map_name = map_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map")
            .to_lowercase();
        let combined_path = output_dir.join(format!("{map_name}.obj"));
        obj::write_combined_obj(&combined_path, &combined_soups)?;
        tracing::info!(
            path = %combined_path.display(),
            chunks = chunks_with_geometry,
            triangles = total_triangles,
            "extract_map: wrote combined OBJ"
        );
    }

    tracing::info!(
        chunks_with_geometry,
        total_triangles,
        total_actors_resolved,
        total_actors_unresolved,
        "extract_map: done"
    );

    Ok(())
}

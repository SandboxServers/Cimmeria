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
pub mod coverage;
pub mod floor_probe;
pub mod geometry;
pub mod nav_roundtrip;
pub mod obj;
pub mod staticmesh;
pub mod transform;
pub mod umap;

use std::path::Path;

use crate::coverage::{ChunkCoverage, MapCoverage};

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
    extract_map_with_report(
        map_dir,
        output_dir,
        ExtractOptions {
            index,
            ..Default::default()
        },
    )
    .map(|_| ())
}

/// Knobs for [`extract_map_with_report`].
///
/// A struct rather than four positional arguments because three of the
/// four are optional and one of them (`combined_obj`) is easy to get
/// dangerously wrong — see its doc.
// No `Debug`: `PackageIndex` doesn't implement it, and a 2.8M-entry
// index is not something you want in a log line anyway.
#[derive(Default, Clone, Copy)]
pub struct ExtractOptions<'a> {
    /// Cross-package export index used to resolve `StaticMeshActor` →
    /// `StaticMesh` references. `None` runs in degraded mode: actors
    /// are walked and counted, but no triangles are emitted.
    pub index: Option<&'a cimmeria_upk_objects::PackageIndex>,
    /// Keep only chunks whose filename contains this substring
    /// (case-insensitive) — useful for iterating on one interior tile
    /// without re-walking 144 chunks.
    pub chunk_filter: Option<&'a str>,
    /// Where to write the single whole-map OBJ, if anywhere. Default
    /// `None`: no combined file.
    ///
    /// **It must not land in `output_dir`.** NavBuilder's `chunked`
    /// mode globs `*.obj` and derives each file's chunk bounds from a
    /// `<hex8>o` stem; a file that doesn't match leaves the bounds
    /// uninitialised, the build dies with "Failed to create
    /// heightfield", nothing is written — and NavBuilder still exits 0.
    /// [`extract_map_with_report`] rejects a path inside `output_dir`
    /// rather than let that happen silently.
    pub combined_obj: Option<&'a Path>,
}

/// [`extract_map`] plus a machine-readable per-chunk coverage report.
///
/// The returned [`MapCoverage`] answers the question the phase table
/// can't: how many `StaticMeshActor`s resolved, why the rest didn't, and
/// how many exports of classes we *don't* decode (Terrain, BSP,
/// BlockingVolume, …) are sitting in each chunk.
pub fn extract_map_with_report(
    map_dir: &Path,
    output_dir: &Path,
    opts: ExtractOptions<'_>,
) -> Result<MapCoverage> {
    let ExtractOptions {
        index,
        chunk_filter,
        combined_obj,
    } = opts;
    let started = std::time::Instant::now();
    tracing::info!(map_dir = %map_dir.display(), output_dir = %output_dir.display(), "extract_map: starting");

    if let Some(combined) = combined_obj {
        if combined.parent() == Some(output_dir) {
            return Err(ExtractError::Other(format!(
                "combined OBJ {} would sit in the per-chunk output directory; \
                 NavBuilder's chunked mode then fails to create a heightfield \
                 and exits 0 with no output. Pick a directory outside {}.",
                combined.display(),
                output_dir.display()
            )));
        }
    }

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    let all_chunks = umap::enumerate_chunks(map_dir)?;
    let enumerated = all_chunks.len();
    let filter_lower = chunk_filter.map(|f| f.to_lowercase());
    let chunks: Vec<_> = all_chunks
        .into_iter()
        .filter(|p| match &filter_lower {
            None => true,
            Some(f) => p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_lowercase().contains(f.as_str()))
                .unwrap_or(false),
        })
        .collect();
    tracing::info!(
        enumerated,
        selected = chunks.len(),
        "extract_map: enumerated chunks"
    );

    let map_name = map_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("map")
        .to_string();

    let mut report = MapCoverage {
        map_name,
        chunks_filtered_out: enumerated - chunks.len(),
        ..Default::default()
    };

    // Combined OBJ accumulator — emitted alongside the per-chunk files
    // so the NavBuilder operator can pick `whole` mode if they want to
    // build a single navmesh from the entire map without chunk
    // boundaries. The filename `<mapname>.obj` mirrors what the legacy
    // C++ extractor produced.
    let mut combined_soups: Vec<geometry::TriangleSoup> = Vec::new();

    for chunk_path in chunks {
        let id = chunk_id::ChunkId::from_umap_path(&chunk_path)?;
        tracing::debug!(
            chunk = %chunk_path.display(),
            chunk_id = format!("{:08x}", id.raw()),
            position_x = id.position_x(),
            position_z = id.position_z(),
            "extract_map: processing chunk"
        );

        // One open per chunk: the LZO decompression is the expensive
        // part, and both the class census and the geometry walk need it.
        let pkg = cimmeria_upk::Package::open(&chunk_path)?;
        let class_census = coverage::census_export_classes(&pkg);
        let exports_total = pkg.exports.len() as u64;

        // Phase 1.2: StaticMesh extraction.
        let mut extraction = staticmesh::extract_chunk_from_package(&pkg, index);
        // Tag the soup with a group so NavBuilder can debug-print which
        // chunk a triangle came from. `Chunk_*` keeps it distinct from
        // the reserved `Terrain_*` prefix NavBuilder skips.
        extraction.soup.group = Some(format!("Chunk_{:08x}", id.raw()));

        // Phase 1.3: Terrain extraction. For each `Terrain` export,
        // parse the tagged-property block, then decode the binary
        // trailer (Heights → InfoData → AlphaXSize → AlphaYSize →
        // WeightedTextureMaps → WeightMapTextures), triangulate via
        // `geometry::triangulate_terrain`. The recipe is documented in
        // `.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`.
        // TODO: wire `Terrain` exports into `extraction.soup` (Phase 1.3).

        // Phase 1.4: BSP Model/Polys — deferred; needs Ghidra trace.

        let mut row = ChunkCoverage {
            chunk: chunk_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string(),
            chunk_id: id.raw(),
            position_x: id.position_x(),
            position_z: id.position_z(),
            exports_total,
            actors_total: extraction.actors_total as u64,
            actors_resolved: extraction.actors_resolved as u64,
            skips: extraction.skips,
            triangles_emitted: extraction.triangles_emitted as u64,
            obj_bytes: 0,
            archetype_actors: extraction.archetype_actors,
            archetype_actors_resolved: extraction.archetype_actors_resolved,
            prefab_outer_actors: extraction.prefab_outer_actors,
            class_census,
        };

        if !row.is_balanced() {
            // Loud on purpose: an unbalanced row means an actor left the
            // walker without a tally, so every "coverage = N%" number
            // downstream is understated by an unknown amount.
            tracing::warn!(
                chunk = %row.chunk,
                actors_total = row.actors_total,
                actors_resolved = row.actors_resolved,
                skips_total = row.skips.total(),
                "extract_map: coverage accounting does not balance"
            );
        }

        if extraction.soup.triangle_count() == 0 {
            // Empty chunks are skipped entirely — no OBJ written. The
            // directory listing surfaces "missing chunks" on its own
            // (a chunk_id with no .obj file means no geometry was
            // resolvable); a zero-triangle stub would just litter the
            // output dir with content-free files.
            tracing::debug!(
                chunk_id = format!("{:08x}", id.raw()),
                actors_total = extraction.actors_total,
                actors_resolved = extraction.actors_resolved,
                actors_unresolved = extraction.actors_unresolved,
                "extract_map: chunk produced no geometry; skipping OBJ write"
            );
            report.chunks.push(row);
            continue;
        }

        let obj_path = output_dir.join(id.obj_filename());
        obj::write_obj(&obj_path, &extraction.soup)?;
        row.obj_bytes = std::fs::metadata(&obj_path).map(|m| m.len()).unwrap_or(0);

        tracing::info!(
            chunk_id = format!("{:08x}", id.raw()),
            actors_total = extraction.actors_total,
            actors_resolved = extraction.actors_resolved,
            triangles = extraction.triangles_emitted,
            path = %obj_path.display(),
            "extract_map: wrote chunk OBJ"
        );

        report.chunks.push(row);
        combined_soups.push(extraction.soup);
    }

    if let (Some(combined_path), false) = (combined_obj, combined_soups.is_empty()) {
        // Opt-in only, and never into `output_dir` — the guard at the
        // top of this function enforces that, because a stray `*.obj`
        // there silently kills NavBuilder's chunked build.
        if let Some(parent) = combined_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        obj::write_combined_obj(combined_path, &combined_soups)?;
        report.combined_obj_bytes = std::fs::metadata(combined_path)
            .map(|m| m.len())
            .unwrap_or(0);
        tracing::info!(
            path = %combined_path.display(),
            chunks = report.chunks_with_geometry(),
            bytes = report.combined_obj_bytes,
            "extract_map: wrote combined OBJ"
        );
    }

    report.elapsed_secs = started.elapsed().as_secs_f64();

    let totals = report.totals();
    tracing::info!(
        chunks_with_geometry = report.chunks_with_geometry(),
        total_triangles = totals.triangles_emitted,
        total_actors_resolved = totals.actors_resolved,
        total_actors_unresolved = totals.skips.total(),
        elapsed_secs = report.elapsed_secs,
        "extract_map: done"
    );

    Ok(report)
}

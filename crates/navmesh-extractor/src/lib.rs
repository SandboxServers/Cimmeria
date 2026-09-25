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
//! **Phase 1.2 (StaticMesh + StaticMeshActor extraction)** and
//! **Phase 1.3 (Terrain decode, holes honoured)** and **Phase 1.4 (BSP
//! `Model` world geometry)**.

pub mod bsp;
pub mod chunk_id;
pub mod cover;
pub mod coverage;
pub mod floor_probe;
pub mod geometry;
pub mod nav_components;
pub mod nav_roundtrip;
pub mod nav_tiled;
pub mod obj;
pub mod obj_slab;
pub mod occluder;
pub mod staticmesh;
pub mod terrain;
/// Synthetic UE3 package fixtures. Behind `test-support` so nothing
/// here reaches a release binary; see the module docs for why the
/// builder lives in this crate rather than `cimmeria-upk`.
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
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
/// Ships **Phase 1.2 (StaticMesh)**, **1.3 (Terrain)** and **1.4
/// (BSP)**. Terrain and BSP need no index, so degraded mode still
/// emits the ground and the level geometry.
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
    /// Leave `Terrain` exports out of the OBJs. Default `false`. Only
    /// useful for measuring one geometry source in isolation — a map
    /// built without its terrain has no ground.
    pub skip_terrain: bool,
    /// Leave BSP `Model` geometry out of the OBJs. Default `false`. Same
    /// purpose as `skip_terrain`; Castle's interior floors are BSP, so a
    /// build without it has rooms with walls and no floor.
    pub skip_bsp: bool,
    /// Keep the buried outer skin of an enclosing CSG hull. Default
    /// `false` — it is dropped, see [`bsp::hull_cap`]. Set it to measure
    /// what the filter is worth; on Castle it puts a 121,041 m²
    /// unreachable walkable sheet back into the mesh.
    ///
    /// Implied by `skip_terrain`: without terrain there is no evidence
    /// that anything is buried, so nothing is dropped either way.
    pub keep_hull_caps: bool,
    /// Walk `InterpActor` exports as `StaticMeshActor`-shaped geometry.
    /// Default `false` — opt-in. `KActor` and `FracturedStaticMeshActor`
    /// are always walked regardless of this flag; see
    /// [`staticmesh::MESH_ACTOR_CLASSES`]'s doc for why `InterpActor` is
    /// the one gated: in this content it is disproportionately doors,
    /// gates, lifts and elevators, and a mover's cooked pose is its
    /// design-time resting state (usually closed), not necessarily
    /// where a player experiences it at runtime. Baking a closed door
    /// into a `.nav` seals the doorway. See
    /// `docs/engine/navmesh-build-pipeline.md` §11 for which maps were
    /// built with this on.
    pub include_interp_actors: bool,
}

/// Collapse `.` and `..` textually. No filesystem access, so it cannot
/// be fooled into the wrong answer by a missing directory — and cannot
/// resolve a symlink either.
fn lexical_clean(path: &Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Resolve a directory path to something two spellings of the same
/// directory compare equal on.
///
/// `canonicalize` is the real answer — it resolves `.`, `..`, symlinks
/// and (on Windows) case — but it requires the whole path to exist.
///
/// When it doesn't, a purely lexical cleanup is **not** a safe fallback
/// for a containment comparison: it returns a *relative* path, and a
/// relative path can never compare equal to the absolute path
/// `canonicalize` gave the other side. That is a real bypass, not a
/// theoretical one — with `output_dir = out`, the combined OBJ
/// `out/not-yet-created/../whole.obj` has a parent
/// (`out/not-yet-created/..`) that cannot be canonicalized, so the old
/// fallback compared relative `out` against an absolute `…/out`, let the
/// path through, and then `create_dir_all` + write resolved it straight
/// back into the chunk directory.
///
/// So: clean lexically first, then **anchor** the result by
/// canonicalizing the longest prefix that does exist and re-appending
/// the rest. Both sides of the comparison then live in the same space.
/// An empty path means "the current directory", which is what
/// `Path::parent` returns for a bare filename.
fn normalize_dir(path: &Path) -> std::path::PathBuf {
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    if let Ok(real) = path.canonicalize() {
        return real;
    }

    let clean = lexical_clean(path);
    let clean = if clean.as_os_str().is_empty() {
        std::path::PathBuf::from(".")
    } else {
        clean
    };

    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    let mut probe = clean.clone();
    loop {
        if let Ok(real) = probe.canonicalize() {
            let mut out = real;
            for name in rest.iter().rev() {
                out.push(name);
            }
            return out;
        }
        let Some(name) = probe.file_name().map(|n| n.to_os_string()) else {
            // A root, or a leading `..` we cannot climb past: nothing
            // left to anchor against.
            return clean;
        };
        rest.push(name);
        if !probe.pop() || probe.as_os_str().is_empty() {
            // `parent()` of a bare relative name is `""`, i.e. the cwd.
            probe = std::path::PathBuf::from(".");
        }
    }
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
        skip_terrain,
        skip_bsp,
        keep_hull_caps,
        include_interp_actors,
    } = opts;
    let mut terrain_totals = terrain::TerrainStats::default();
    let (mut bsp_models_failed, mut bsp_triangles) = (0usize, 0usize);
    let (mut bsp_hull_cap_triangles, mut bsp_hull_cap_area_m2) = (0usize, 0.0f64);
    let started = std::time::Instant::now();
    tracing::info!(map_dir = %map_dir.display(), output_dir = %output_dir.display(), "extract_map: starting");

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    // Containment guard, after `create_dir_all` so `canonicalize` on
    // `output_dir` can resolve. A lexical `parent() == Some(out)`
    // comparison is bypassed by any equivalent spelling —
    // `out` vs `./out/x.obj`, or `build/out` vs
    // `build/x/../out/x.obj` — and the bypass is silent all the way
    // through NavBuilder, which exits 0 with no output.
    if let Some(combined) = combined_obj {
        let parent = combined.parent().unwrap_or(Path::new(""));
        if normalize_dir(parent) == normalize_dir(output_dir) {
            return Err(ExtractError::Other(format!(
                "combined OBJ {} would sit in the per-chunk output directory; \
                 NavBuilder's chunked mode then fails to create a heightfield \
                 and exits 0 with no output. Pick a directory outside {}.",
                combined.display(),
                output_dir.display()
            )));
        }
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
        include_interp_actors,
        ..Default::default()
    };

    // Combined OBJ accumulator — emitted alongside the per-chunk files
    // so the NavBuilder operator can pick `whole` mode if they want to
    // build a single navmesh from the entire map without chunk
    // boundaries. The filename `<mapname>.obj` mirrors what the legacy
    // C++ extractor produced.
    let mut combined_soups: Vec<geometry::TriangleSoup> = Vec::new();

    // Memo of prefab-archetype path -> mesh key, shared across every
    // chunk. Castle's 961 archetype-stub actors share 86 distinct
    // archetype paths, so hoisting this out of the per-chunk walk is
    // what keeps prefab-package opens down to those 86.
    let mut archetype_cache = staticmesh::ArchetypeCache::default();

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
        let mut extraction = staticmesh::extract_chunk_from_package(
            &pkg,
            index,
            &mut archetype_cache,
            include_interp_actors,
        );
        // Tag the soup with a group so NavBuilder can debug-print which
        // chunk a triangle came from. `Chunk_*` keeps it distinct from
        // the reserved `Terrain_*` prefix NavBuilder skips.
        extraction.soup.group = Some(format!("Chunk_{:08x}", id.raw()));

        // Phase 1.3: Terrain extraction. Holes (`TID_Visibility_Off`
        // quads) are honoured, so building footprints stay open for the
        // interior floors to fill.
        let terrain_first_face = extraction.soup.triangle_count();
        let terrain_stats = if skip_terrain {
            terrain::TerrainStats::default()
        } else {
            terrain::collect_terrain_triangles(&pkg, &mut extraction.soup)
        };
        // The BSP hull-cap filter needs to know where the ground is, and
        // the ground is the triangles we have just pushed. Built from the
        // soup range rather than a second decode pass.
        let terrain_ceiling = (!keep_hull_caps)
            .then(|| {
                bsp::TerrainCeiling::from_triangles(
                    extraction
                        .soup
                        .triangles_in(terrain_first_face..extraction.soup.triangle_count()),
                )
            })
            .flatten();
        if terrain_stats.parse_failures > 0 {
            // Missing ground is never silently acceptable: a chunk whose
            // terrain failed to decode produces a navmesh with a hole the
            // size of the chunk.
            tracing::warn!(
                chunk_id = format!("{:08x}", id.raw()),
                terrain_actors = terrain_stats.terrain_actors,
                parse_failures = terrain_stats.parse_failures,
                "extract_map: terrain export failed to decode; ground geometry missing"
            );
        }
        terrain_totals.terrain_actors += terrain_stats.terrain_actors;
        terrain_totals.parse_failures += terrain_stats.parse_failures;
        terrain_totals.quads_total += terrain_stats.quads_total;
        terrain_totals.quads_holed += terrain_stats.quads_holed;
        terrain_totals.triangles_emitted += terrain_stats.triangles_emitted;

        // Phase 1.4: BSP. The level `Model` is where Castle's interior
        // floors live; trigger volumes are excluded inside the collector.
        let bsp_stats = if skip_bsp {
            bsp::BspStats::default()
        } else {
            bsp::collect_bsp_triangles(
                &pkg,
                &mut extraction.soup,
                bsp::BspOptions {
                    terrain_ceiling: terrain_ceiling.as_ref(),
                },
            )
        };
        if bsp_stats.models_failed > 0 {
            // The deserializer enforces exact consumption, so a failure
            // is a decoder bug and the chunk is missing floors or walls.
            tracing::warn!(
                chunk_id = format!("{:08x}", id.raw()),
                models_failed = bsp_stats.models_failed,
                errors = ?bsp_stats.parse_errors,
                "extract_map: BSP Model failed to decode; world geometry missing"
            );
        }
        bsp_models_failed += bsp_stats.models_failed;
        bsp_triangles += bsp_stats.triangles_emitted;
        bsp_hull_cap_triangles += bsp_stats.hull_cap_triangles_excluded;
        bsp_hull_cap_area_m2 += bsp_stats.hull_cap_area_m2;

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
            // Everything that lands in the chunk's OBJ, all sources.
            triangles_emitted: extraction.soup.triangle_count() as u64,
            staticmesh_triangles: extraction.triangles_emitted as u64,
            terrain_triangles: terrain_stats.triangles_emitted as u64,
            terrain_quads_holed: terrain_stats.quads_holed as u64,
            terrain_parse_failures: terrain_stats.parse_failures as u64,
            bsp_triangles: bsp_stats.triangles_emitted as u64,
            bsp_hull_cap_triangles: bsp_stats.hull_cap_triangles_excluded as u64,
            bsp_models_failed: bsp_stats.models_failed as u64,
            obj_bytes: 0,
            archetype_actors: extraction.archetype_actors,
            archetype_actors_resolved: extraction.archetype_actors_resolved,
            prefab_outer_actors: extraction.prefab_outer_actors,
            actors_resolved_via_archetype: extraction.actors_resolved_via_archetype,
            triangles_via_archetype: extraction.triangles_via_archetype as u64,
            prefab_packages_opened: extraction.prefab_packages_opened,
            class_census,
        };

        if !row.sources_balance() {
            // A source pushed into the soup without tallying. Every
            // "source X contributes N%" number below is then wrong, and
            // so is the OBJ-vs-report cross-check the integration test
            // relies on.
            tracing::warn!(
                chunk = %row.chunk,
                triangles_emitted = row.triangles_emitted,
                staticmesh_triangles = row.staticmesh_triangles,
                terrain_triangles = row.terrain_triangles,
                bsp_triangles = row.bsp_triangles,
                "extract_map: per-source triangle tallies do not sum to the soup total"
            );
        }

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
    let (arch_hits, arch_misses) = archetype_cache.stats();
    tracing::info!(
        chunks_with_geometry = report.chunks_with_geometry(),
        total_triangles = totals.triangles_emitted,
        total_actors_resolved = totals.actors_resolved,
        total_actors_unresolved = totals.skips.total(),
        actors_via_archetype = totals.actors_resolved_via_archetype,
        triangles_via_archetype = totals.triangles_via_archetype,
        archetype_paths = archetype_cache.len(),
        archetype_cache_hits = arch_hits,
        archetype_cache_misses = arch_misses,
        elapsed_secs = report.elapsed_secs,
        terrain_actors = terrain_totals.terrain_actors,
        terrain_parse_failures = terrain_totals.parse_failures,
        terrain_quads_holed = terrain_totals.quads_holed,
        terrain_triangles = terrain_totals.triangles_emitted,
        bsp_models_failed,
        bsp_triangles,
        bsp_hull_cap_triangles,
        bsp_hull_cap_area_m2,
        "extract_map: done"
    );

    Ok(report)
}

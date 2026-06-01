//! Phase 1.2 acceptance test: walk every Castle_CellBlock chunk and
//! confirm the StaticMeshActor parse produces a plausible instance
//! count.
//!
//! This is the integration test that the brief calls for. It runs in
//! "index-less" mode because building the full `PackageIndex` takes
//! ~50 seconds — too long for a unit test. The check it CAN make
//! without the index is the actor-walk step: how many
//! `StaticMeshActor` exports does each chunk hold, and does the
//! tagged-property decode produce a non-empty mesh reference for them?
//!
//! The deep dive cites chunk `fffefffd` (a dense chunk) as having
//! **469 StaticMeshActors / 533 StaticMeshComponents / 39 Brushes**.
//! We don't pin the exact count (asset bundles drift slightly between
//! locales), but we DO confirm the chunk-walk produces a reasonable
//! total — at least one chunk with ≥100 actors and the whole map
//! exceeding a few thousand actors across 65 chunks.
//!
//! Self-skips when the cooked asset bundle isn't present.

use std::path::{Path, PathBuf};

use cimmeria_navmesh_extractor::staticmesh::{collect_static_mesh_instances, extract_chunk};
use cimmeria_navmesh_extractor::umap::enumerate_chunks;

/// Where the cooked Castle_CellBlock chunks live in the dev environment.
///
/// The SGW asset tree (`sgw/Stargate Worlds-QA/…`) is kept as a sibling
/// of the Cimmeria repo, but this crate may be tested from either the
/// main repo checkout or a worktree under `.claude/worktrees/<slug>/`.
/// Rather than hardcode a fixed climb count, walk ancestors until we
/// find a directory that contains the expected `sgw/` sibling.
fn castle_cellblock_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suffix =
        PathBuf::from("sgw/Stargate Worlds-QA/Working/SGWGame/CookedPC/Maps/Castle_CellBlock");

    // Probe up to 10 ancestors — generous enough for any worktree
    // nesting depth we're likely to use.
    for ancestor in manifest.ancestors().take(10) {
        let candidate = ancestor.join(&suffix);
        if candidate.exists() {
            return candidate;
        }
    }

    // Fall back to the "obvious" 3-level climb so the skip message
    // names the path we WOULD have looked at.
    manifest
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.join(&suffix))
        .unwrap_or_else(|| manifest.join(&suffix))
}

fn skip_if_missing(dir: &Path, what: &str) -> bool {
    if !dir.exists() {
        eprintln!(
            "Skipping {what} — asset bundle not present at {}",
            dir.display()
        );
        return true;
    }
    false
}

#[test]
fn castle_cellblock_walks_static_mesh_actors() {
    let dir = castle_cellblock_dir();
    if skip_if_missing(&dir, "castle_cellblock_walks_static_mesh_actors") {
        return;
    }

    // Enumerate every .umap chunk via the production walker so the test
    // sees exactly what the orchestrator does. Using `enumerate_chunks`
    // (instead of a raw `read_dir + extension == "umap"` filter) drops
    // the master streaming-level `<MapName>.umap` that has no hex suffix
    // and would otherwise be counted alongside the real chunks.
    let chunks = enumerate_chunks(&dir).expect("enumerate_chunks");

    assert!(
        chunks.len() >= 60,
        "Castle_CellBlock should ship ~65 chunks; found {}",
        chunks.len()
    );

    let mut total_actors = 0usize;
    let mut max_chunk_actors = 0usize;
    let mut chunks_with_actors = 0usize;

    for chunk_path in &chunks {
        let pkg = match cimmeria_upk::Package::open(chunk_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to open {}: {e}", chunk_path.display());
                continue;
            }
        };
        let instances = collect_static_mesh_instances(&pkg);
        let count = instances.len();
        total_actors += count;
        if count > 0 {
            chunks_with_actors += 1;
        }
        if count > max_chunk_actors {
            max_chunk_actors = count;
        }
    }

    eprintln!(
        "Castle_CellBlock: {} chunks scanned, {} chunks with resolvable StaticMeshActors, \
         {} total resolvable instances, max {} in a single chunk",
        chunks.len(),
        chunks_with_actors,
        total_actors,
        max_chunk_actors
    );

    // Sanity: at least one chunk should produce ≥100 RESOLVED instances
    // — these are actors whose `StaticMeshComponent → StaticMesh`
    // reference is recoverable without prefab archetype traversal.
    // The dense chunk `fffefffd` lands at 375 resolved out of 469 raw
    // actors in the asset bundle the test ran against; archetype-based
    // instances (the missing ~94) are a Phase 1.2-extension target.
    assert!(
        max_chunk_actors >= 100,
        "No chunk had ≥100 resolvable StaticMeshActor instances; densest was \
         {max_chunk_actors}. Either the asset bundle is unexpectedly sparse, \
         the actor walker is broken, or the StaticMeshComponent tagged-property \
         offset has changed (expected 8 bytes past serial start)."
    );
    assert!(
        total_actors >= 1000,
        "Total resolvable instance count {total_actors} is implausibly low \
         for Castle_CellBlock."
    );
}

/// Look for a pre-built `PackageIndex` cache that a developer can drop
/// into the project root. Returns `None` if absent — tests that need
/// the index then self-skip.
fn try_load_package_index() -> Option<cimmeria_upk_objects::PackageIndex> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors().take(10) {
        for name in [
            "package_index.bin",
            "package_index.bincode",
            ".package_index.bin",
        ] {
            let candidate = ancestor.join(name);
            if candidate.exists() {
                match cimmeria_upk_objects::PackageIndex::load(&candidate) {
                    Ok(idx) => {
                        eprintln!("Loaded PackageIndex from {}", candidate.display());
                        return Some(idx);
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to load PackageIndex from {}: {e}",
                            candidate.display()
                        );
                    }
                }
            }
        }
    }
    None
}

#[test]
fn castle_cellblock_with_index_produces_triangles() {
    // End-to-end Phase 1.2 test: if the developer has dropped a cached
    // PackageIndex at the project root, run the full chunk extractor
    // against the dense chunk fffefffd and confirm we emit triangles.
    // Otherwise self-skip — building the index takes ~50 seconds and
    // is too heavy for CI.
    let dir = castle_cellblock_dir();
    if skip_if_missing(&dir, "castle_cellblock_with_index_produces_triangles") {
        return;
    }
    let Some(index) = try_load_package_index() else {
        eprintln!(
            "Skipping castle_cellblock_with_index_produces_triangles — no \
             cached PackageIndex (run `cargo run --bin build-package-index -- \
             <CookedPC dir> package_index.bincode` from the repo root to build one)"
        );
        return;
    };

    let chunk = dir.join("Castle_CellBlock-fffefffd.umap");
    if !chunk.exists() {
        eprintln!("Skipping — chunk fffefffd missing from this asset bundle");
        return;
    }

    let result = extract_chunk(&chunk, Some(&index)).expect("extract_chunk");
    eprintln!(
        "Castle_CellBlock fffefffd: actors_total={} resolved={} unresolved={} triangles={}",
        result.actors_total,
        result.actors_resolved,
        result.actors_unresolved,
        result.triangles_emitted
    );

    // At minimum we expect a non-trivial number of resolved instances
    // (Phase 1.2 baseline against this chunk is ~375).
    assert!(
        result.actors_resolved >= 200,
        "Expected ≥200 resolved actors in dense chunk fffefffd; got {}",
        result.actors_resolved
    );
    // And the triangle soup must be non-empty.
    assert!(
        result.soup.triangle_count() >= 10_000,
        "Expected ≥10k triangles from chunk fffefffd; got {}",
        result.soup.triangle_count()
    );
}

#[test]
fn castle_cellblock_extract_chunk_without_index_emits_no_geometry() {
    // Regression guard for the degraded-mode path: when no PackageIndex
    // is supplied the extractor must walk actors, return a non-zero
    // `actors_total`, and produce a zero-triangle soup. The earlier
    // implementation tried to emit triangles anyway and panicked when
    // the cross-package lookup failed.
    let dir = castle_cellblock_dir();
    if skip_if_missing(
        &dir,
        "castle_cellblock_extract_chunk_without_index_emits_no_geometry",
    ) {
        return;
    }

    // Pick the first chunk deterministically. `enumerate_chunks` already
    // sorts and filters out the non-chunk master `.umap`.
    let chunks = enumerate_chunks(&dir).expect("enumerate_chunks");
    let chunk = chunks.first().expect("at least one chunk");

    let result = extract_chunk(chunk, None).expect("extract_chunk");
    assert_eq!(
        result.soup.triangle_count(),
        0,
        "index-less mode must emit zero triangles"
    );
    // With no index, every actor is unresolved.
    assert_eq!(result.actors_resolved, 0);
    assert_eq!(result.actors_unresolved, result.actors_total);
}

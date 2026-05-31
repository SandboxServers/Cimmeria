//! Phase 1.2 end-to-end: drive `extract_map` against the full
//! Castle_CellBlock chunk directory and verify the OBJ output is
//! plausible.
//!
//! Self-skips when either the cooked asset bundle is missing or when no
//! cached `PackageIndex` is on disk. The package-index build takes ~50s
//! so we never construct one inside the test — the CI environment is
//! expected to either ship a cache or skip this layer of the test
//! pyramid.

use std::path::PathBuf;

use cimmeria_navmesh_extractor::extract_map;
use cimmeria_upk_objects::PackageIndex;

fn castle_cellblock_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suffix =
        PathBuf::from("sgw/Stargate Worlds-QA/Working/SGWGame/CookedPC/Maps/Castle_CellBlock");
    for ancestor in manifest.ancestors().take(10) {
        let candidate = ancestor.join(&suffix);
        if candidate.exists() {
            return candidate;
        }
    }
    manifest.join(&suffix)
}

fn try_load_package_index() -> Option<PackageIndex> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors().take(10) {
        for name in [
            "package_index.bin",
            "package_index.bincode",
            ".package_index.bin",
        ] {
            let candidate = ancestor.join(name);
            if candidate.exists() {
                if let Ok(idx) = PackageIndex::load(&candidate) {
                    return Some(idx);
                }
            }
        }
    }
    None
}

/// Unique temp directory per test thread — the round-trip test crate
/// follows this pattern; we match it so concurrent runs don't collide.
fn unique_tempdir(prefix: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "{prefix}-{}-{:?}-{}",
        std::process::id(),
        std::thread::current().id(),
        nanos,
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn extract_map_castle_cellblock_emits_chunk_obj_files() {
    let map_dir = castle_cellblock_dir();
    if !map_dir.exists() {
        eprintln!(
            "Skipping extract_map_castle_cellblock_emits_chunk_obj_files — \
             asset bundle not present at {}",
            map_dir.display()
        );
        return;
    }
    let Some(index) = try_load_package_index() else {
        eprintln!(
            "Skipping extract_map_castle_cellblock_emits_chunk_obj_files — \
             no cached PackageIndex. Build one with \
             `cargo run --bin build-package-index -- <CookedPC> \
             --output package_index.bincode` from the repo root."
        );
        return;
    };

    let out_dir = unique_tempdir("cimmeria-navmesh-extract-map");
    extract_map(&map_dir, &out_dir, Some(&index)).expect("extract_map");

    // Inventory the output.
    let mut obj_files: Vec<_> = std::fs::read_dir(&out_dir)
        .expect("read output dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "obj").unwrap_or(false))
        .collect();
    obj_files.sort();

    let chunk_objs = obj_files
        .iter()
        .filter(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.len() == 9 && s.ends_with('o'))
                .unwrap_or(false)
        })
        .count();
    let combined_objs = obj_files
        .iter()
        .filter(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s == "castle_cellblock")
                .unwrap_or(false)
        })
        .count();

    eprintln!(
        "extract_map output: {} total OBJ ({} per-chunk, {} combined)",
        obj_files.len(),
        chunk_objs,
        combined_objs
    );

    // We expect SOME per-chunk OBJs (the resolvable subset) and exactly
    // one combined map-level OBJ.
    assert!(
        chunk_objs >= 5,
        "Expected ≥5 per-chunk OBJ files; got {chunk_objs}"
    );
    assert_eq!(
        combined_objs, 1,
        "Expected exactly one combined map-level OBJ"
    );

    // Combined OBJ should be hefty — the dense chunk alone produces
    // ~85k triangles, and 65 chunks combined will land in the hundreds
    // of thousands. Cross-check against the file size as a rough proxy:
    // each triangle line in the OBJ is ~30 bytes (`f a b c\n`), so the
    // combined OBJ should be at least a few MB.
    let combined_path = out_dir.join("castle_cellblock.obj");
    let combined_meta = std::fs::metadata(&combined_path).expect("combined.obj exists");
    assert!(
        combined_meta.len() >= 1_000_000,
        "Combined OBJ at {} is only {} bytes — extraction may be incomplete",
        combined_path.display(),
        combined_meta.len()
    );

    // Quick sanity scan: every OBJ must start with the extractor's
    // header comment so a future reader (or git diff) recognises it.
    for obj in &obj_files {
        let head = std::fs::read_to_string(obj)
            .ok()
            .and_then(|s| s.lines().next().map(|l| l.to_string()))
            .unwrap_or_default();
        assert!(
            head.starts_with("# Generated by cimmeria-navmesh-extractor"),
            "{} missing extractor signature header, first line was: {head:?}",
            obj.display()
        );
    }
}

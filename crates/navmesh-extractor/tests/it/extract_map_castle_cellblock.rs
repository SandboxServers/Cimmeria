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
    // An explicit cache path wins: the index is ~190 MB, so developers
    // keep it out of the repo tree.
    if let Ok(p) = std::env::var("CIMMERIA_PACKAGE_INDEX") {
        return PackageIndex::load(PathBuf::from(p).as_path()).ok();
    }
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

/// Count `f ` lines in an OBJ without loading it — the dense chunks run
/// to tens of megabytes.
fn count_obj_faces(path: &std::path::Path) -> usize {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("open {} for face count: {e}", path.display()));
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| l.starts_with("f "))
        .count()
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

    // One extraction only. A prior `extract_map` pass into the same
    // directory rewrote every OBJ the reported run then wrote, doubling
    // the runtime of an already heavy asset test and — worse — leaving
    // behind any file the reported run did *not* write, which the
    // `obj_files.len() == chunk_objs` guard below would then count as
    // if the second run had produced it. The thin `extract_map`
    // wrapper is covered asset-free in `extract_map_synthetic.rs`.
    let out_dir = unique_tempdir("cimmeria-navmesh-extract-map");
    let combined_dir = unique_tempdir("cimmeria-navmesh-extract-map-combined");
    let combined_path = combined_dir.join("castle_cellblock.obj");
    let report = cimmeria_navmesh_extractor::extract_map_with_report(
        &map_dir,
        &out_dir,
        cimmeria_navmesh_extractor::ExtractOptions {
            index: Some(&index),
            chunk_filter: None,
            combined_obj: Some(&combined_path),
            ..Default::default()
        },
    )
    .expect("extract_map_with_report");

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
    eprintln!(
        "extract_map output: {} total OBJ ({} per-chunk)",
        obj_files.len(),
        chunk_objs
    );

    assert!(
        chunk_objs >= 5,
        "Expected ≥5 per-chunk OBJ files; got {chunk_objs}"
    );

    // NOTHING but `<hex8>o.obj` may sit in the per-chunk directory.
    // NavBuilder's chunked mode globs `*.obj` and derives chunk bounds
    // from the stem; a whole-map `castle_cellblock.obj` next to them
    // leaves those bounds uninitialised, the build fails with "Failed
    // to create heightfield", and NavBuilder still exits 0. The
    // combined OBJ is opt-in and goes to its own directory.
    assert_eq!(
        obj_files.len(),
        chunk_objs,
        "non-chunk OBJ in the per-chunk output dir: {:?}",
        obj_files
            .iter()
            .filter(|p| p
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| !(s.len() == 9 && s.ends_with('o')))
                .unwrap_or(true))
            .collect::<Vec<_>>()
    );

    // The opt-in combined OBJ landed in its own directory and is hefty
    // — the dense chunk alone produces ~85k triangles, and 65 chunks
    // combined land in the hundreds of thousands.
    let combined_meta = std::fs::metadata(&combined_path).expect("combined OBJ exists");
    assert!(
        combined_meta.len() >= 1_000_000,
        "Combined OBJ at {} is only {} bytes — extraction may be incomplete",
        combined_path.display(),
        combined_meta.len()
    );

    // The per-source accounting invariant, checked against the bytes on
    // disk rather than against itself: for every chunk,
    //
    //   f-lines in the OBJ == triangles_emitted
    //                      == staticmesh + terrain + bsp
    //
    // The coverage TSV is how anyone answers "how much of this map is
    // BSP", so a source that lands in the soup without a tally silently
    // rewrites that answer. Hull-cap triangles are deliberately absent
    // from both sides: they never reach the soup.
    let mut checked = 0usize;
    for row in &report.chunks {
        assert!(
            row.sources_balance(),
            "{}: triangles_emitted {} != {} StaticMesh + {} Terrain + {} BSP",
            row.chunk,
            row.triangles_emitted,
            row.staticmesh_triangles,
            row.terrain_triangles,
            row.bsp_triangles
        );
        if row.triangles_emitted == 0 {
            continue;
        }
        let obj = out_dir.join(format!("{:08x}o.obj", row.chunk_id));
        let faces = count_obj_faces(&obj);
        assert_eq!(
            faces as u64, row.triangles_emitted,
            "{}: the OBJ carries {faces} faces but the report claims {}",
            row.chunk, row.triangles_emitted
        );
        checked += 1;
    }
    assert!(
        checked >= 5,
        "only {checked} chunk(s) had geometry to check"
    );
    let totals = report.totals();
    eprintln!(
        "per-source totals: {} = {} StaticMesh + {} Terrain + {} BSP \
         ({} hull-cap triangles dropped, {} terrain holes, {} terrain \
         parse failures, {} Model decode failures) over {checked} chunks",
        totals.triangles_emitted,
        totals.staticmesh_triangles,
        totals.terrain_triangles,
        totals.bsp_triangles,
        totals.bsp_hull_cap_triangles,
        totals.terrain_quads_holed,
        totals.terrain_parse_failures,
        totals.bsp_models_failed,
    );
    assert!(
        totals.sources_balance(),
        "the TOTAL row must balance too: {} != {} + {} + {}",
        totals.triangles_emitted,
        totals.staticmesh_triangles,
        totals.terrain_triangles,
        totals.bsp_triangles
    );

    // Quick sanity scan: every OBJ must start with the extractor's
    // header comment so a future reader (or git diff) recognises it.
    // Use a BufReader and read just the first line — the combined OBJ
    // can be tens of MB and `read_to_string` would load the whole file
    // just to look at the header.
    use std::io::{BufRead, BufReader};
    for obj in &obj_files {
        let file = std::fs::File::open(obj).expect("open OBJ for header check");
        let mut head = String::new();
        BufReader::new(file)
            .read_line(&mut head)
            .expect("read OBJ first line");
        // Strip trailing newline(s) so the assert message stays tidy.
        let head = head.trim_end_matches(['\r', '\n']);
        assert!(
            head.starts_with("# Generated by cimmeria-navmesh-extractor"),
            "{} missing extractor signature header, first line was: {head:?}",
            obj.display()
        );
    }
}

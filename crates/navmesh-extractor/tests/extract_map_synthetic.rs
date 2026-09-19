//! End-to-end `extract_map_with_report` over synthetic chunks.
//!
//! Everything here runs in CI. The cooked client tree can never be
//! committed, so the asset-backed integration tests self-skip on the
//! runner and the orchestrator they exercise reports zero coverage;
//! these build the same object graph from
//! [`cimmeria_navmesh_extractor::test_support`] and drive the real
//! walkers over it.
//!
//! What they are *not*: a replacement for the Castle tests. A
//! synthetic chunk proves the orchestrator's plumbing — filtering,
//! source tallies, OBJ shape, the refusals — and cannot prove anything
//! about what is actually inside a shipped `.umap`. The asset tests
//! keep that job.

use std::path::{Path, PathBuf};

use cimmeria_navmesh_extractor::test_support::{
    index_over, mesh_package, scratch_dir, ChunkFixture, ModelPayload, StaticMeshPayload,
    TerrainPayload,
};
use cimmeria_navmesh_extractor::{extract_map, extract_map_with_report, ExtractOptions};

/// A flat BSP quad spanning one 100 cm square at height `z`, wound the
/// way a real node pool is (stored normal up).
fn floor_quad(z: f32) -> ModelPayload {
    ModelPayload::horizontal_quad(0.0, 100.0, 0.0, 100.0, z, 0, [0.0, 0.0, 1.0])
}

/// Three chunks: a dense one (terrain + BSP + a resolvable
/// StaticMeshActor), a terrain-only one, and one with no geometry at
/// all. Returns `(map_dir, content_dir)`.
fn three_chunk_map(tag: &str) -> (PathBuf, PathBuf) {
    let root = scratch_dir(tag);
    let content = root.join("Content");
    let maps = root.join("Maps").join("Synth");
    std::fs::create_dir_all(&content).unwrap();
    std::fs::create_dir_all(&maps).unwrap();

    mesh_package(
        &content,
        "Fx-Props",
        "Fx-Floor01",
        &StaticMeshPayload::unit_triangle(),
    );

    let mut dense = ChunkFixture::new();
    dense.add_terrain(&TerrainPayload::flat(2, 2));
    dense.add_level_model(&floor_quad(50.0));
    dense.add_static_mesh_actor("Floor_0", [0.0, 0.0, 0.0], 1.0, ("Fx-Props", "Fx-Floor01"));
    dense.write(&maps, "Synth", 0x0000_0001);

    let mut terrain_only = ChunkFixture::new();
    terrain_only.add_terrain(&TerrainPayload::flat(1, 1).at([1000.0, 0.0, 0.0]));
    terrain_only.write(&maps, "Synth", 0x0000_0002);

    let empty = ChunkFixture::new();
    empty.write(&maps, "Synth", 0x0000_0003);

    (maps, content)
}

fn obj_stems(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "obj").unwrap_or(false))
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn extract_map_writes_one_obj_per_chunk_with_geometry_and_skips_the_empty_one() {
    let (maps, content) = three_chunk_map("extract-three-chunk");
    let index = index_over(&content);
    let out = scratch_dir("extract-three-chunk-out");

    let report = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            index: Some(&index),
            ..Default::default()
        },
    )
    .expect("extract");

    assert_eq!(report.chunks.len(), 3, "all three chunks were walked");
    assert_eq!(report.chunks_with_geometry(), 2);
    // The empty chunk must leave no file behind: NavBuilder globs
    // `*.obj` and a content-free stub is pure noise in the listing an
    // operator reads to spot missing chunks.
    assert_eq!(
        obj_stems(&out),
        vec!["00000001o.obj".to_string(), "00000002o.obj".to_string()]
    );
    assert_eq!(report.chunks_filtered_out, 0);

    // Per-chunk source tallies, exactly. The dense chunk: 8 terrain
    // triangles (2x2 patches, no holes), 2 BSP (one quad), 1 StaticMesh
    // (the unit triangle).
    let dense = report.chunks.iter().find(|c| c.chunk_id == 1).unwrap();
    assert_eq!(dense.terrain_triangles, 8);
    assert_eq!(dense.bsp_triangles, 2);
    assert_eq!(dense.staticmesh_triangles, 1);
    assert_eq!(dense.triangles_emitted, 11);
    assert!(dense.sources_balance(), "{dense:?}");
    assert!(dense.is_balanced());
    assert_eq!(dense.actors_total, 1);
    assert_eq!(dense.actors_resolved, 1);
    assert_eq!(dense.terrain_parse_failures, 0);
    assert_eq!(dense.bsp_models_failed, 0);
    assert!(dense.obj_bytes > 0);

    let terrain_only = report.chunks.iter().find(|c| c.chunk_id == 2).unwrap();
    assert_eq!(terrain_only.terrain_triangles, 2, "1x1 patch = 2 triangles");
    assert_eq!(terrain_only.bsp_triangles, 0);
    assert_eq!(terrain_only.triangles_emitted, 2);

    let empty = report.chunks.iter().find(|c| c.chunk_id == 3).unwrap();
    assert_eq!(empty.triangles_emitted, 0);
    assert_eq!(empty.obj_bytes, 0, "no OBJ was written, so no bytes");

    let totals = report.totals();
    assert_eq!(totals.triangles_emitted, 13);
    assert!(totals.sources_balance());
}

#[test]
fn the_emitted_obj_swaps_y_and_z_uses_crlf_and_carries_one_face_per_triangle() {
    // The three properties NavBuilder silently punishes: the wrong
    // column order rasterises floors as walls, LF endings drop the last
    // index of single-digit face lines, and a missing face is a hole.
    let (maps, content) = three_chunk_map("extract-obj-shape");
    let index = index_over(&content);
    let out = scratch_dir("extract-obj-shape-out");
    extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            index: Some(&index),
            chunk_filter: Some("00000002"),
            ..Default::default()
        },
    )
    .expect("extract");

    let obj = std::fs::read_to_string(out.join("00000002o.obj")).expect("chunk OBJ");
    assert!(
        !obj.contains('\n') || obj.matches("\r\n").count() == obj.matches('\n').count(),
        "every LF must be part of a CRLF"
    );
    assert!(obj.ends_with("\r\n"));
    assert!(obj.contains("o Chunk_00000002\r\n"), "{obj}");

    // Terrain is at world Z = 0 and spans X 1000..1100, Y 0..100. The
    // writer emits `v <ue.X> <ue.Z> <ue.Y>`, so the UE3 Y extent must
    // appear in the *third* column and the constant Z in the second.
    let verts: Vec<&str> = obj.lines().filter(|l| l.starts_with("v ")).collect();
    assert_eq!(verts.len(), 6, "2 triangles, no shared vertices");
    assert!(verts.contains(&"v 1000 0 0"), "{verts:?}");
    assert!(verts.contains(&"v 1100 0 100"), "{verts:?}");
    assert!(
        !verts.iter().any(|v| v.ends_with(" 0 0 100")),
        "raw (X, Y, Z) order would put the Y extent in column two: {verts:?}"
    );

    let faces: Vec<&str> = obj.lines().filter(|l| l.starts_with("f ")).collect();
    assert_eq!(faces, vec!["f 1 2 3", "f 4 5 6"]);
}

#[test]
fn chunk_filter_selects_by_filename_substring_and_reports_the_rest_as_filtered() {
    let (maps, content) = three_chunk_map("extract-filter");
    let index = index_over(&content);
    let out = scratch_dir("extract-filter-out");
    let report = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            index: Some(&index),
            chunk_filter: Some("00000001"),
            ..Default::default()
        },
    )
    .expect("extract");

    assert_eq!(report.chunks.len(), 1);
    assert_eq!(report.chunks_filtered_out, 2);
    assert_eq!(obj_stems(&out), vec!["00000001o.obj".to_string()]);
}

#[test]
fn skip_terrain_and_skip_bsp_remove_exactly_their_own_source() {
    let (maps, content) = three_chunk_map("extract-skips");
    let index = index_over(&content);

    let no_terrain = extract_map_with_report(
        &maps,
        &scratch_dir("extract-skips-noterrain"),
        ExtractOptions {
            index: Some(&index),
            skip_terrain: true,
            ..Default::default()
        },
    )
    .expect("extract");
    let t = no_terrain.totals();
    assert_eq!(t.terrain_triangles, 0);
    assert_eq!(t.bsp_triangles, 2, "BSP is untouched by skip_terrain");
    assert_eq!(t.staticmesh_triangles, 1);
    assert_eq!(t.triangles_emitted, 3);
    assert!(t.sources_balance());

    let no_bsp = extract_map_with_report(
        &maps,
        &scratch_dir("extract-skips-nobsp"),
        ExtractOptions {
            index: Some(&index),
            skip_bsp: true,
            ..Default::default()
        },
    )
    .expect("extract");
    let t = no_bsp.totals();
    assert_eq!(t.bsp_triangles, 0);
    assert_eq!(t.terrain_triangles, 10, "8 dense + 2 terrain-only");
    assert_eq!(t.triangles_emitted, 11);
    assert!(t.sources_balance());
}

#[test]
fn degraded_mode_walks_actors_and_tallies_them_without_emitting_staticmesh() {
    // What CI itself sees: no PackageIndex. Terrain and BSP still have
    // to come through, or a runner without the asset bundle would
    // report an empty map and nobody would notice the difference
    // between "no index" and "no geometry".
    let (maps, _content) = three_chunk_map("extract-degraded");
    let out = scratch_dir("extract-degraded-out");
    let report = extract_map_with_report(&maps, &out, ExtractOptions::default()).expect("extract");

    let t = report.totals();
    assert_eq!(t.staticmesh_triangles, 0);
    assert_eq!(t.terrain_triangles, 10);
    assert_eq!(t.bsp_triangles, 2);
    assert_eq!(t.triangles_emitted, 12);
    assert!(t.sources_balance());
    assert_eq!(t.actors_total, 1, "the actor is still walked and counted");
    assert_eq!(t.actors_resolved, 0);
    assert_eq!(
        t.skips
            .get(cimmeria_navmesh_extractor::coverage::SkipReason::NoPackageIndex),
        1
    );
    assert!(t.is_balanced());
}

#[test]
fn extract_map_is_the_same_walk_as_extract_map_with_report() {
    // The thin wrapper has its own output directory here, so the file
    // set it produces is asserted on its own rather than inherited
    // from a second run into the same place.
    let (maps, content) = three_chunk_map("extract-wrapper");
    let index = index_over(&content);
    let out = scratch_dir("extract-wrapper-out");
    extract_map(&maps, &out, Some(&index)).expect("extract_map");
    assert_eq!(
        obj_stems(&out),
        vec!["00000001o.obj".to_string(), "00000002o.obj".to_string()]
    );
    assert!(
        !out.join("Synth.obj").exists(),
        "the wrapper must not write a combined OBJ"
    );
}

#[test]
fn the_combined_obj_lands_outside_the_output_dir_and_renumbers_indices() {
    let (maps, content) = three_chunk_map("extract-combined");
    let index = index_over(&content);
    let out = scratch_dir("extract-combined-out");
    let elsewhere = scratch_dir("extract-combined-whole");
    let combined = elsewhere.join("synth.obj");

    let report = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            index: Some(&index),
            combined_obj: Some(&combined),
            ..Default::default()
        },
    )
    .expect("extract");

    assert!(report.combined_obj_bytes > 0);
    assert_eq!(
        obj_stems(&out),
        vec!["00000001o.obj".to_string(), "00000002o.obj".to_string()],
        "the combined OBJ must not appear in the per-chunk directory"
    );
    let text = std::fs::read_to_string(&combined).unwrap();
    let faces: Vec<&str> = text.lines().filter(|l| l.starts_with("f ")).collect();
    assert_eq!(faces.len(), 13, "11 dense + 2 terrain-only");
    // The second soup's first face must be rebased past the first
    // soup's 33 vertices, not restart at 1.
    assert!(faces.contains(&"f 34 35 36"), "{faces:?}");
}

#[test]
fn a_combined_obj_inside_the_output_dir_is_refused() {
    let (maps, _content) = three_chunk_map("extract-refuse");
    let out = scratch_dir("extract-refuse-out");
    let err = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            combined_obj: Some(&out.join("synth.obj")),
            ..Default::default()
        },
    )
    .expect_err("must refuse");
    assert!(
        format!("{err}").contains("per-chunk output directory"),
        "unexpected error: {err}"
    );
    assert!(
        obj_stems(&out).is_empty(),
        "the refusal must happen before anything is written"
    );
}

#[test]
fn an_equivalent_spelling_of_the_output_dir_is_also_refused() {
    // The bug shape: `out` and `out/../out/synth.obj` name the same
    // directory, but a lexical `parent() == Some(out)` comparison says
    // they don't. NavBuilder's chunked mode then globs the stray
    // `synth.obj`, fails to create a heightfield, writes nothing, and
    // still exits 0 -- the exact silent failure this guard exists for.
    let (maps, _content) = three_chunk_map("extract-refuse-alias");
    let out = scratch_dir("extract-refuse-alias-out");
    let alias = out.join("..").join(out.file_name().unwrap()).join("s.obj");
    let err = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            combined_obj: Some(&alias),
            ..Default::default()
        },
    )
    .expect_err("an aliased path must be refused too");
    assert!(
        format!("{err}").contains("per-chunk output directory"),
        "unexpected error: {err}"
    );
    assert!(!out.join("s.obj").exists());
}

#[test]
fn a_combined_obj_in_a_sibling_directory_is_allowed() {
    // The near-miss: a directory whose name merely *starts* with the
    // output directory's name is a different directory.
    let (maps, _content) = three_chunk_map("extract-sibling");
    let out = scratch_dir("extract-sibling-out");
    let sibling = out.with_file_name(format!(
        "{}-whole",
        out.file_name().unwrap().to_string_lossy()
    ));
    let report = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            combined_obj: Some(&sibling.join("synth.obj")),
            ..Default::default()
        },
    )
    .expect("a sibling directory is fine");
    assert!(report.combined_obj_bytes > 0);
    let _ = std::fs::remove_dir_all(&sibling);
}

// ----- hull-cap filter -----

/// A chunk whose BSP is an enclosing block: a top skin, a bottom skin
/// and an interior floor between them, under terrain that proves the
/// two skins are buried.
fn buried_hull_chunk(dir: &Path) -> PathBuf {
    let mut model = ModelPayload::default();
    // Top skin at z = 0, facing up.
    model.push_quad(
        [
            [0.0, 0.0, 0.0],
            [200.0, 0.0, 0.0],
            [200.0, 200.0, 0.0],
            [0.0, 200.0, 0.0],
        ],
        0,
        [0.0, 0.0, 1.0],
    );
    // Bottom skin at z = -500, facing down.
    model.push_quad(
        [
            [0.0, 0.0, -500.0],
            [200.0, 0.0, -500.0],
            [200.0, 200.0, -500.0],
            [0.0, 200.0, -500.0],
        ],
        0,
        [0.0, 0.0, -1.0],
    );
    // The real interior floor, on neither plane.
    model.push_quad(
        [
            [0.0, 0.0, -300.0],
            [200.0, 0.0, -300.0],
            [200.0, 200.0, -300.0],
            [0.0, 200.0, -300.0],
        ],
        0,
        [0.0, 0.0, 1.0],
    );

    let mut chunk = ChunkFixture::new();
    // Terrain 400 cm above the top skin, over the same XY footprint.
    chunk.add_terrain(&TerrainPayload::flat(2, 2).at([0.0, 0.0, 400.0]));
    chunk.add_level_model(&model);
    chunk.write(dir, "Synth", 0x0000_0009)
}

#[test]
fn the_buried_hull_skin_is_dropped_and_the_interior_floor_is_kept() {
    // The filter's whole purpose: the top skin of the enclosing CSG
    // block rasterises into a large unreachable walkable sheet, and
    // the terrain above it is the evidence that nothing can stand
    // there. The interior floor at the same normal direction must
    // survive, or the filter has eaten the geometry it exists to
    // protect.
    let maps = scratch_dir("hullcap-map");
    buried_hull_chunk(&maps);
    let out = scratch_dir("hullcap-out");
    let report = extract_map_with_report(&maps, &out, ExtractOptions::default()).expect("extract");

    let c = &report.chunks[0];
    assert_eq!(c.terrain_triangles, 8);
    assert_eq!(
        c.bsp_hull_cap_triangles, 4,
        "both skins (2 quads x 2 triangles) are buried"
    );
    assert_eq!(c.bsp_triangles, 2, "only the interior floor survives");
    assert!(c.sources_balance());
    // Every surviving BSP vertex sits on the interior floor plane.
    let obj = std::fs::read_to_string(out.join("00000009o.obj")).unwrap();
    assert!(
        !obj.contains(" -500\r\n")
            && !obj.lines().any(|l| l.starts_with("v ")
                && l.ends_with(" 0 0")
                && l.split_whitespace().nth(2) == Some("0")),
        "no cap vertex may remain: {obj}"
    );
}

#[test]
fn keep_hull_caps_puts_the_buried_skin_back() {
    let maps = scratch_dir("hullcap-keep-map");
    buried_hull_chunk(&maps);
    let out = scratch_dir("hullcap-keep-out");
    let report = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            keep_hull_caps: true,
            ..Default::default()
        },
    )
    .expect("extract");

    let c = &report.chunks[0];
    assert_eq!(c.bsp_hull_cap_triangles, 0);
    assert_eq!(c.bsp_triangles, 6, "all three quads");
    assert!(c.sources_balance());
}

#[test]
fn skip_terrain_disables_the_hull_cap_filter() {
    // Documented in `ExtractOptions::keep_hull_caps`: without terrain
    // there is no evidence anything is buried, so nothing may be
    // dropped. A filter that fired anyway would delete real floors
    // from a terrain-free measurement run.
    let maps = scratch_dir("hullcap-noterrain-map");
    buried_hull_chunk(&maps);
    let out = scratch_dir("hullcap-noterrain-out");
    let report = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            skip_terrain: true,
            ..Default::default()
        },
    )
    .expect("extract");

    let c = &report.chunks[0];
    assert_eq!(c.terrain_triangles, 0);
    assert_eq!(c.bsp_hull_cap_triangles, 0);
    assert_eq!(c.bsp_triangles, 6);
}

// ----- coverage report -----

#[test]
fn the_coverage_tsv_carries_one_row_per_chunk_plus_totals_with_per_source_columns() {
    let (maps, content) = three_chunk_map("coverage-tsv");
    let index = index_over(&content);
    let out = scratch_dir("coverage-tsv-out");
    let report = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            index: Some(&index),
            ..Default::default()
        },
    )
    .expect("extract");

    let mut buf = Vec::new();
    report.write_tsv_into(&mut buf).unwrap();
    let text = String::from_utf8(buf).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
    let rows: Vec<Vec<&str>> = lines.map(|l| l.split('\t').collect()).collect();
    assert_eq!(rows.len(), 4, "three chunks plus a totals row");

    let col = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .unwrap_or_else(|| panic!("no {name} column in {header:?}"))
    };
    let dense = rows.iter().find(|r| r[0] == "Synth-00000001").unwrap();
    assert_eq!(dense[col("staticmesh_triangles")], "1");
    assert_eq!(dense[col("terrain_triangles")], "8");
    assert_eq!(dense[col("bsp_triangles")], "2");
    assert_eq!(dense[col("triangles")], "11");
    assert_eq!(dense[col("sources_balanced")], "yes");
    assert_eq!(dense[col("balanced")], "yes");
    assert_eq!(dense[col("terrain_parse_failures")], "0");
    assert_eq!(dense[col("bsp_models_failed")], "0");

    // Every row must be balanced, including the totals row -- an
    // unbalanced total is how an untallied geometry source hides.
    for row in &rows {
        assert_eq!(row[col("sources_balanced")], "yes", "{row:?}");
        assert_eq!(row[col("balanced")], "yes", "{row:?}");
    }
    let totals = rows.last().unwrap();
    assert_eq!(totals[col("triangles")], "13");
}

#[test]
fn the_class_census_counts_every_export_class_in_the_map() {
    let (maps, content) = three_chunk_map("census-tsv");
    let index = index_over(&content);
    let out = scratch_dir("census-tsv-out");
    let report = extract_map_with_report(
        &maps,
        &out,
        ExtractOptions {
            index: Some(&index),
            ..Default::default()
        },
    )
    .expect("extract");

    let mut buf = Vec::new();
    report.write_class_census_into(&mut buf).unwrap();
    let text = String::from_utf8(buf).unwrap();
    let row = |class: &str| -> Vec<String> {
        text.lines()
            .find(|l| l.starts_with(&format!("{class}\t")))
            .unwrap_or_else(|| panic!("no {class} row in:\n{text}"))
            .split('\t')
            .map(|s| s.to_string())
            .collect()
    };
    // Two chunks carry a Terrain; one carries a Model and one
    // StaticMeshActor.
    assert_eq!(row("Terrain")[1], "2");
    assert_eq!(row("Terrain")[2], "2", "present in two chunks");
    assert_eq!(row("Model")[1], "1");
    assert_eq!(row("StaticMeshActor")[1], "1");
    assert_eq!(row("Level")[1], "3", "every chunk has a PersistentLevel");
}

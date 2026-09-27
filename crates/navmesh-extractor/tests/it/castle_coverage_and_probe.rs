//! End-to-end over ONE Castle interior chunk: extract, check the
//! coverage accounting, then floor-probe the emitted OBJ.
//!
//! Self-skips (loudly) when the cooked asset bundle or a cached
//! `PackageIndex` is absent — same contract as
//! `tests/it/extract_map_castle_cellblock.rs`. A skipped run is not a pass;
//! the skip reason is printed to stderr.
//!
//! Override the discovery with:
//!
//! ```text
//! CIMMERIA_COOKED_PC=<...>/SGWGame/CookedPC
//! CIMMERIA_PACKAGE_INDEX=<...>/package_index.bin
//! ```
//!
//! `Castle-000a0002` is the Interrogation Block interior tile the
//! 2026-09-18 colo playtest walked through — 844 `StaticMeshActor`
//! exports, and the chunk holding the Zuritska cell probe point.

use std::path::PathBuf;

use cimmeria_navmesh_extractor::floor_probe::{
    AxisMapping, Confidence, ProbeConfig, ProbePoint, ProbeRun,
};
use cimmeria_navmesh_extractor::{extract_map_with_report, obj, ExtractOptions};
use cimmeria_upk_objects::PackageIndex;

/// The chunk under test, and the world point inside it.
const CHUNK: &str = "000a0002";
/// Zuritska cell, from playtest telemetry (BigWorld units, y up).
const ZURITSKA_CELL: [f32; 3] = [268.0, 66.79, 1042.59];

fn cooked_pc_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CIMMERIA_COOKED_PC") {
        let p = PathBuf::from(p);
        return p.is_dir().then_some(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suffix = PathBuf::from("sgw/Stargate Worlds-QA/Working/SGWGame/CookedPC");
    manifest
        .ancestors()
        .take(10)
        .map(|a| a.join(&suffix))
        .find(|c| c.is_dir())
}

fn try_load_package_index() -> Option<PackageIndex> {
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
fn castle_interior_chunk_coverage_and_axis_mapping() {
    let Some(cooked) = cooked_pc_dir() else {
        eprintln!(
            "SKIPPED castle_interior_chunk_coverage_and_axis_mapping — \
             cooked asset bundle not found. Set CIMMERIA_COOKED_PC to the \
             SGWGame/CookedPC directory to run it."
        );
        return;
    };
    let map_dir = cooked.join("Maps").join("Castle");
    if !map_dir.is_dir() {
        eprintln!(
            "SKIPPED castle_interior_chunk_coverage_and_axis_mapping — \
             no Maps/Castle under {}",
            cooked.display()
        );
        return;
    }
    let Some(index) = try_load_package_index() else {
        eprintln!(
            "SKIPPED castle_interior_chunk_coverage_and_axis_mapping — \
             no cached PackageIndex. Build one with \
             `cargo run -p cimmeria-upk-objects --release --bin build-package-index \
             -- <CookedPC> --output package_index.bin` and point \
             CIMMERIA_PACKAGE_INDEX at it."
        );
        return;
    };

    let out = unique_tempdir("cimmeria-castle-coverage");
    let report = extract_map_with_report(
        &map_dir,
        &out,
        ExtractOptions {
            index: Some(&index),
            chunk_filter: Some(CHUNK),
            combined_obj: None,
            ..Default::default()
        },
    )
    .expect("extract_map_with_report");

    // ---- coverage accounting ----

    assert_eq!(
        report.chunks.len(),
        1,
        "chunk filter {CHUNK:?} should select exactly one chunk, got {:?}",
        report.chunks.iter().map(|c| &c.chunk).collect::<Vec<_>>()
    );
    assert!(
        report.chunks_filtered_out >= 100,
        "Castle ships 144 chunks; only {} were filtered out",
        report.chunks_filtered_out
    );

    let chunk = &report.chunks[0];
    eprintln!(
        "{}: {} exports, {} StaticMeshActor, {} resolved, {} triangles, \
         {} archetype-instanced ({} resolved), ModelComponent={} Terrain={}",
        chunk.chunk,
        chunk.exports_total,
        chunk.actors_total,
        chunk.actors_resolved,
        chunk.triangles_emitted,
        chunk.archetype_actors,
        chunk.archetype_actors_resolved,
        chunk.class_count("ModelComponent"),
        chunk.class_count("Terrain"),
    );

    assert!(
        chunk.is_balanced(),
        "coverage accounting must balance: {} actors, {} resolved, {} skipped",
        chunk.actors_total,
        chunk.actors_resolved,
        chunk.skips.total()
    );
    assert!(
        chunk.actors_total >= 500,
        "Castle-{CHUNK} should carry ~844 StaticMeshActor exports; found {}",
        chunk.actors_total
    );
    assert!(
        chunk.actors_resolved * 10 >= chunk.actors_total * 6,
        "resolved {} of {} actors — below the 60% floor the StaticMesh path \
         has held since Phase 1.2",
        chunk.actors_resolved,
        chunk.actors_total
    );
    assert!(
        chunk.triangles_emitted >= 50_000,
        "only {} triangles from a dense interior tile",
        chunk.triangles_emitted
    );

    // The finding this whole spike turns on: an interior tile carries
    // BSP surfaces (`ModelComponent`) and terrain that the extractor
    // does NOT decode. If this ever reads zero, either the census
    // broke or someone shipped a decoder and this assertion is the
    // reminder to re-measure the floor coverage.
    assert!(
        chunk.class_count("ModelComponent") > 0,
        "expected built BSP surfaces in the Interrogation Block tile"
    );
    assert!(
        chunk.class_count("Terrain") > 0,
        "every Castle chunk carries a Terrain actor"
    );

    // ---- axis mapping ----
    //
    // Deliberately NOT asserting "a floor exists under the Zuritska
    // cell" — the measured answer is that it does not, because the
    // floor is BSP. What IS assertable is that the CA05 mapping puts
    // the chunk's geometry *around* that point while NavBuilder's
    // current raw-UE3 swizzle puts it tens of units away.

    let obj_path = out.join(format!("{CHUNK}o.obj"));
    let soup = obj::read_obj_as_ue3(&obj_path).expect("read back the emitted OBJ");
    assert_eq!(
        soup.triangle_count() as u64,
        chunk.triangles_emitted,
        "OBJ round-trip lost triangles"
    );

    // The OBJ NavBuilder reads must be CRLF and Y/Z-swapped. Both are
    // silent failures downstream: LF drops faces whose last index is
    // one digit, and raw (X, Y, Z) rasterises every floor as a wall.
    let raw = std::fs::read(&obj_path).expect("read OBJ bytes");
    let lf_only = raw
        .windows(2)
        .filter(|w| w[1] == b'\n' && w[0] != b'\r')
        .count();
    assert_eq!(lf_only, 0, "emitted OBJ has bare LF line endings");
    let obj_space = obj::read_obj(&obj_path).expect("read OBJ verbatim");
    assert_eq!(
        obj_space.vertices[0],
        obj::ue3_to_obj(soup.vertices[0]),
        "OBJ vertices are not Y/Z-swapped"
    );

    let point = vec![ProbePoint::new(
        "Zuritska_cell",
        Confidence::High,
        ZURITSKA_CELL,
        "playtest telemetry",
    )];
    let cfg = ProbeConfig::default();

    let mut ca05 = ProbeRun::new(AxisMapping::CA05, cfg, point.clone());
    ca05.add_soup(&soup);
    let mut navbuilder = ProbeRun::new(AxisMapping::NAVBUILDER_ON_RAW_UE3, cfg, point);
    navbuilder.add_soup(&soup);

    let a = &ca05.results()[0];
    let b = &navbuilder.results()[0];
    eprintln!(
        "Zuritska cell: +Y+Z+X nearest={:?} near_tris={} near_vertical={} | \
         +Z+Y+X nearest={:?} near_tris={}",
        a.nearest_dist, a.near_tris, a.near_vertical, b.nearest_dist, b.near_tris
    );

    let ca05_dist = a.nearest_dist.expect("CA05 mapping saw some geometry");
    assert!(
        ca05_dist < 5.0,
        "+Y+Z+X should land this chunk's geometry within a few units of a \
         point a player stood on; nearest was {ca05_dist}"
    );
    assert!(
        a.near_tris > 100,
        "expected the cell's walls and props in the neighbourhood; got {}",
        a.near_tris
    );
    assert!(
        a.near_vertical > 0,
        "an interrogation cell should have wall triangles around the point"
    );

    let nb_dist = b
        .nearest_dist
        .expect("NavBuilder mapping saw some geometry");
    assert!(
        nb_dist > 25.0,
        "+Z+Y+X (NavBuilder's loadOBJ swizzle applied to raw UE3 cm) should \
         misplace the geometry badly; nearest was {nb_dist}. If this now \
         passes, the OBJ writer's coordinate convention changed and the \
         README's axis section needs re-deriving."
    );

    let _ = std::fs::remove_dir_all(&out);
}

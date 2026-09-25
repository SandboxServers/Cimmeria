//! The occluder walk and the `occluder_extract` binary over synthetic
//! packages (NA27). CI has no client assets; the asset-backed numbers come
//! from `occluder_extract measure` runs recorded in the data README.

use std::path::PathBuf;
use std::process::{Command, Output};

use cimmeria_navmesh_extractor::nav_roundtrip::XrcNav;
use cimmeria_navmesh_extractor::occluder::for_each_chunk;
use cimmeria_navmesh_extractor::test_support::{
    index_over, mesh_package, scratch_dir, ChunkFixture, ModelPayload, StaticMeshPayload,
    TerrainPayload,
};
use cimmeria_navmesh_extractor::{extract_map_with_report, ExtractOptions};
use cimmeria_occluder::{BuildParams, LayerKind, OccluderBuilder, PagedOccluder, Sight, Source};

/// A `CookedPC`-shaped tree with one map: a 2 x 2 m terrain (1 m patches
/// at the origin), a BSP ceiling 3 m up over the first square metre, and a
/// resolvable StaticMeshActor; plus a terrain-only chunk. Returns the
/// cooked root.
fn cooked_root(tag: &str) -> PathBuf {
    let root = scratch_dir(tag);
    let maps = root.join("Maps").join("Synth");
    std::fs::create_dir_all(&maps).unwrap();
    mesh_package(
        &root,
        "Fx-Props",
        "Fx-Floor01",
        &StaticMeshPayload::unit_triangle(),
    );
    let mut dense = ChunkFixture::new();
    dense.add_terrain(&TerrainPayload::flat(2, 2));
    dense.add_level_model(&ModelPayload::horizontal_quad(
        0.0,
        100.0,
        0.0,
        100.0,
        300.0,
        0,
        [0.0, 0.0, 1.0],
    ));
    dense.add_static_mesh_actor("Floor_0", [0.0, 0.0, 0.0], 1.0, ("Fx-Props", "Fx-Floor01"));
    dense.write(&maps, "Synth", 0x0000_0001);
    let mut terrain_only = ChunkFixture::new();
    terrain_only.add_terrain(&TerrainPayload::flat(1, 1).at([3000.0, 0.0, 0.0]));
    terrain_only.write(&maps, "Synth", 0x0000_0002);
    root
}

#[test]
fn the_walk_sees_the_same_triangles_as_extract_map() {
    let root = cooked_root("occ-walk-vs-extract");
    let index = index_over(&root);
    let map = root.join("Maps").join("Synth");
    let out = scratch_dir("occ-walk-vs-extract-out");
    let report = extract_map_with_report(
        &map,
        &out,
        ExtractOptions {
            index: Some(&index),
            ..Default::default()
        },
    )
    .unwrap();
    let totals = report.totals();
    let mut seen = (0usize, 0usize);
    let stats = for_each_chunk(&map, Some(&index), false, |c| {
        seen.0 += c.geometry.len();
        seen.1 += c.terrain.len();
    })
    .unwrap();
    assert!(totals.triangles_emitted > 0);
    assert_eq!(
        stats.staticmesh_triangles as u64,
        totals.staticmesh_triangles
    );
    assert_eq!(stats.terrain_triangles as u64, totals.terrain_triangles);
    assert_eq!(stats.bsp_triangles as u64, totals.bsp_triangles);
    assert_eq!(seen.0, stats.staticmesh_triangles + stats.bsp_triangles);
    assert_eq!(seen.1, stats.terrain_triangles);
}

/// The walk's output is BigWorld metres, Y up: the BSP ceiling authored at
/// UE3 z = 300 cm is at BW y = 3, and the second chunk's terrain authored
/// at UE3 x = 3000 cm lies at BW z = 30.
#[test]
fn the_walk_emits_bigworld_metres_y_up() {
    let root = cooked_root("occ-walk-axes");
    let index = index_over(&root);
    let mut geometry = Vec::new();
    let mut terrain = Vec::new();
    for_each_chunk(&root.join("Maps").join("Synth"), Some(&index), false, |c| {
        geometry.extend_from_slice(&c.geometry);
        terrain.extend_from_slice(&c.terrain);
    })
    .unwrap();
    assert!(
        geometry
            .iter()
            .any(|t| t.iter().all(|v| (v[1] - 3.0).abs() < 1e-4)),
        "the BSP ceiling at 3 m"
    );
    assert!(
        terrain
            .iter()
            .any(|t| t.iter().all(|v| (v[2] - 30.0).abs() <= 1.0 + 1e-4)),
        "the second terrain at BW z = 30"
    );
}

#[test]
fn an_occluder_built_from_the_walk_blocks_at_the_ceiling_and_keeps_terrain_exact() {
    let root = cooked_root("occ-build");
    let index = index_over(&root);
    let mut b = OccluderBuilder::new(BuildParams::default(), "Synth").unwrap();
    for_each_chunk(&root.join("Maps").join("Synth"), Some(&index), false, |c| {
        for t in &c.geometry {
            b.add_triangle(t, Source::Geometry);
        }
        for t in &c.terrain {
            b.add_triangle(t, Source::Terrain);
        }
    })
    .unwrap();
    assert_eq!(b.terrain_fallback_count(), 0, "1 m patches at the origin");
    let occ = b.finish().unwrap();
    assert!(occ.heightfield().is_some());
    // Straight up through the ceiling.
    assert!(matches!(
        occ.sight([0.5, 1.0, 0.5], [0.6, 5.0, 0.5]),
        Sight::Blocked {
            layer: LayerKind::Geometry,
            ..
        }
    ));
    // Under it, sideways, at eye height: clear.
    assert_eq!(occ.sight([0.2, 1.5, 0.2], [1.8, 1.5, 1.8]), Sight::Clear);
    // Down into the terrain.
    assert!(matches!(
        occ.sight([1.5, 1.0, 1.5], [1.6, -1.0, 1.6]),
        Sight::Blocked {
            layer: LayerKind::Terrain,
            ..
        }
    ));
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_occluder_extract")
}

fn run(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("spawn occluder_extract")
}

#[test]
fn the_build_cli_writes_an_occ_that_loads_back() {
    let root = cooked_root("occ-cli");
    let out = scratch_dir("occ-cli-out");
    let index_path = out.join("index.bin");
    index_over(&root).save(&index_path).unwrap();
    let occ_path = out.join("synth.occ");
    let result = run(&[
        "build",
        "--cooked-root",
        root.to_str().unwrap(),
        "--map",
        "Synth",
        "--index",
        index_path.to_str().unwrap(),
        "--out",
        occ_path.to_str().unwrap(),
    ]);
    assert_eq!(
        result.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let occ = PagedOccluder::load(&occ_path).unwrap();
    assert!(occ.label().starts_with("Synth cell=0.5"), "{}", occ.label());
    assert_eq!(occ.stats().resident_pages, 0, "loading unpacks nothing");
    assert!(occ.stats().pages >= 1);
    // Under the 3 m ceiling at eye height: clear. Up through it: blocked.
    assert_eq!(occ.sight([0.2, 1.5, 0.2], [1.8, 1.5, 1.8]), Sight::Clear);
    assert!(matches!(
        occ.sight([0.5, 1.0, 0.5], [0.6, 5.0, 0.5]),
        Sight::Blocked { .. }
    ));
    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    assert!(stdout.contains("pages"), "{stdout}");
}

/// A 2 x 2 m walkable quad at the origin, as an XRC `.nav`.
fn write_origin_nav(path: &std::path::Path) {
    let nav = XrcNav {
        agent_height: 1.8,
        agent_climb: 0.6,
        agent_radius: 0.6,
        nverts: 4,
        npolys: 1,
        nvp: 4,
        border_size: 0,
        cs: 1.0,
        ch: 1.0,
        bmin: [0.0, 0.0, 0.0],
        bmax: [2.0, 1.0, 2.0],
        verts: vec![0, 0, 0, 2, 0, 0, 2, 0, 2, 0, 0, 2],
        polys: vec![0, 1, 2, 3, 0xffff, 0xffff, 0xffff, 0xffff],
        regs: vec![0],
        flags: vec![1],
        areas: vec![63],
        detail_nmeshes: 0,
        detail_nverts: 0,
        detail_ntris: 0,
        detail_meshes: vec![],
        detail_verts: vec![],
        detail_tris: vec![],
    };
    let mut f = std::fs::File::create(path).unwrap();
    nav.write(&mut f).unwrap();
}

/// With `--nav` the coverage is the explorable area: the far terrain
/// chunk (BW z 30-31, 28 m past the walkable quad) is dropped at a 2 m
/// margin, and a point there reads off the grid.
#[test]
fn the_build_cli_trims_to_the_components_holding_an_entry_point() {
    let root = cooked_root("occ-cli-trim");
    let out = scratch_dir("occ-cli-trim-out");
    let index_path = out.join("index.bin");
    index_over(&root).save(&index_path).unwrap();
    let nav = out.join("synth.nav");
    write_origin_nav(&nav);
    let entry = out.join("entry.tsv");
    std::fs::write(
        &entry,
        "world\tsource\tx\ty\tz\nsynth\tspawnlist:test\t1.0\t0.0\t1.0\n",
    )
    .unwrap();
    let report = out.join("report.tsv");
    let occ_path = out.join("synth.occ");
    let result = run(&[
        "build",
        "--cooked-root",
        root.to_str().unwrap(),
        "--map",
        "Synth",
        "--index",
        index_path.to_str().unwrap(),
        "--out",
        occ_path.to_str().unwrap(),
        "--nav",
        nav.to_str().unwrap(),
        "--entry-points",
        entry.to_str().unwrap(),
        "--margin",
        "2",
        "--report",
        report.to_str().unwrap(),
    ]);
    assert_eq!(
        result.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let occ = PagedOccluder::load(&occ_path).unwrap();
    assert!(occ.covers(1.0, 1.0));
    assert!(
        !occ.covers(0.5, 30.5),
        "the far terrain chunk is trimmed away"
    );
    let text = std::fs::read_to_string(&report).unwrap();
    let rows: Vec<&str> = text.lines().collect();
    let header: Vec<&str> = rows[0].split('\t').collect();
    let row: Vec<&str> = rows[1].split('\t').collect();
    let col = |name: &str| row[header.iter().position(|h| *h == name).unwrap()];
    assert_eq!(col("entry_located"), "1");
    assert_eq!(col("fallback_all_components"), "false");
    assert!(col("trimmed_triangles").parse::<u64>().unwrap() >= 2);
}

/// NA28's tiled `.nav` (`XRCT`) is read too: the same origin quad as a
/// one-tile tiled mesh, and the same trim.
#[test]
fn the_build_cli_trims_from_a_tiled_navmesh() {
    use cimmeria_navmesh_extractor::nav_tiled::{XrcTile, XrcTiledNav};
    let root = cooked_root("occ-cli-trim-tiled");
    let out = scratch_dir("occ-cli-trim-tiled-out");
    let index_path = out.join("index.bin");
    index_over(&root).save(&index_path).unwrap();
    let single = out.join("single.nav");
    write_origin_nav(&single);
    let mesh = XrcNav::read(&mut std::fs::File::open(&single).unwrap()).unwrap();
    let tiled = XrcTiledNav {
        agent_height: mesh.agent_height,
        agent_climb: mesh.agent_climb,
        agent_radius: mesh.agent_radius,
        orig: [0.0, 0.0, 0.0],
        tile_width: 2.0,
        tile_height: 2.0,
        max_tile_polys: 1,
        tiles: vec![XrcTile {
            tile_x: 0,
            tile_y: 0,
            mesh,
        }],
    };
    let nav = out.join("synth.nav");
    tiled
        .write(&mut std::fs::File::create(&nav).unwrap())
        .unwrap();
    assert_eq!(&std::fs::read(&nav).unwrap()[..4], b"XRCT");
    let entry = out.join("entry.tsv");
    std::fs::write(
        &entry,
        "world\tsource\tx\ty\tz\nsynth\tspawnlist:test\t1.0\t0.0\t1.0\n",
    )
    .unwrap();
    let occ_path = out.join("synth.occ");
    let result = run(&[
        "build",
        "--cooked-root",
        root.to_str().unwrap(),
        "--map",
        "Synth",
        "--index",
        index_path.to_str().unwrap(),
        "--out",
        occ_path.to_str().unwrap(),
        "--nav",
        nav.to_str().unwrap(),
        "--entry-points",
        entry.to_str().unwrap(),
        "--margin",
        "2",
    ]);
    assert_eq!(
        result.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let occ = PagedOccluder::load(&occ_path).unwrap();
    assert!(occ.covers(1.0, 1.0));
    assert!(
        !occ.covers(0.5, 30.5),
        "trimmed as from the single-mesh file"
    );
}

#[test]
fn the_cli_refuses_unknown_flags_and_a_missing_index() {
    let out = run(&["build", "--map", "Synth", "--not-a-flag", "x"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--not-a-flag"));
    let dir = scratch_dir("occ-cli-noindex");
    let out = run(&[
        "build",
        "--cooked-root",
        dir.to_str().unwrap(),
        "--map",
        "Synth",
        "--index",
        dir.join("absent.bin").to_str().unwrap(),
        "--out",
        dir.join("x.occ").to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!dir.join("x.occ").exists());
}

//! `NavBuilder tile=<cells>`: the tiled build writes an `XRCT` file whose
//! tiles join back into the surface the single-mesh build would have made.
//!
//! The fixture is one chunk holding an L-shaped floor and a ramp (60 m by
//! 40 m, 3 m of rise), built with `tile=32` (9.6 m tiles at `cs=0.3`), so
//! every part of it crosses several tile seams. If the tiles were built
//! without a border, or the portal edges were not marked, the offline
//! graph (which links portals the way `dtNavMesh::connectExtLinks` does)
//! would fall apart into one component per tile.
//!
//! Self-skips (loudly) when no NavBuilder is found, or when the one found
//! predates `tile=` (the reference `NavBuilder_d.exe` does). Point
//! `CIMMERIA_NAVBUILDER` at a tree build to run it.

use std::path::{Path, PathBuf};
use std::process::Command;

use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_navmesh_extractor::nav_components::NavGraph;
use cimmeria_navmesh_extractor::nav_tiled::NavFile;
use cimmeria_navmesh_extractor::obj::write_obj_into;

/// Chunk grid X = 3, Z = 10: BW x in [300, 400], z in [1000, 1100].
const CHUNK_ID: u32 = 0x000a_0003;

fn navbuilder_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CIMMERIA_NAVBUILDER") {
        return Some(PathBuf::from(p));
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for name in ["NavBuilder.exe", "NavBuilder_d.exe"] {
        for ancestor in manifest.ancestors().take(6) {
            let c = ancestor.join("bin64").join(name);
            if c.exists() {
                return Some(c);
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
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Two triangles over a UE3-XY rectangle, height interpolated along X, in
/// the winding NavBuilder turns into an upward face (see
/// `navbuilder_axis_roundtrip.rs`).
fn floor_quad(x0: f32, y0: f32, x1: f32, y1: f32, z0: f32, z1: f32) -> [[[f32; 3]; 3]; 2] {
    let h = |x: f32| z0 + (z1 - z0) * (x - x0) / (x1 - x0);
    let a = [x0, y0, h(x0)];
    let b = [x1, y0, h(x1)];
    let c = [x1, y1, h(x1)];
    let d = [x0, y1, h(x0)];
    [[a, c, b], [a, d, c]]
}

/// UE3 centimetres; BW = (ue.Y, ue.Z, ue.X) / 100.
fn write_fixture(chunk_dir: &Path) {
    let mut soup = TriangleSoup::new(Some(format!("Chunk_{CHUNK_ID:08x}")));
    let mut quads = Vec::new();
    // Long leg, BW x 300..320, z 1000..1060, y 50.
    quads.extend(floor_quad(
        100_000.0, 30_000.0, 106_000.0, 32_000.0, 5_000.0, 5_000.0,
    ));
    // Stub, BW x 320..340, z 1000..1020.
    quads.extend(floor_quad(
        100_000.0, 32_000.0, 102_000.0, 34_000.0, 5_000.0, 5_000.0,
    ));
    // Ramp, BW z 1060..1075, rising to y 53.
    quads.extend(floor_quad(
        106_000.0, 30_000.0, 107_500.0, 32_000.0, 5_000.0, 5_300.0,
    ));
    // A 3 m x 3 m platform on its own, BW x 347.1..350.1, z 1040..1043.
    // The heightfield starts at the chunk bounds (3, 10) — NavBuilder pads
    // with the chunk *grid* position — so 9.6 m tiles put a seam at
    // x = 3 + 36 * 9.6 = 348.6, through the platform's middle. After the
    // ledge filter and the 0.6 m erosion it is 4 x 4 cells, under the
    // default minRegionSize=8 (64 cells), so a single-mesh build drops it.
    // Each tile sees its half touch the tile border and keeps it; only the
    // seam filter removes it.
    quads.extend(floor_quad(
        104_000.0, 34_710.0, 104_300.0, 35_010.0, 5_000.0, 5_000.0,
    ));
    for t in quads {
        soup.push(t);
    }
    let mut buf = Vec::new();
    write_obj_into(&mut buf, std::slice::from_ref(&soup)).unwrap();
    std::fs::write(chunk_dir.join(format!("{CHUNK_ID:08x}o.obj")), buf).unwrap();
}

/// Runs NavBuilder; `(exit code, stdout, bytes of the .nav if written)`.
fn run(exe: &Path, chunk_dir: &Path, out: &Path, extra: &[&str]) -> (i32, String, Option<Vec<u8>>) {
    let _ = std::fs::remove_file(out);
    let output = Command::new(exe)
        .arg("chunked")
        .arg(chunk_dir)
        .arg(out)
        .arg("nav")
        .args(extra)
        .output()
        .expect("spawn NavBuilder");
    let code = output.status.code().expect("no signal on Windows");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (code, stdout, std::fs::read(out).ok())
}

#[test]
fn a_tiled_build_rejoins_across_its_seams_and_ignores_the_thread_count() {
    let Some(exe) = navbuilder_path() else {
        eprintln!("SKIPPED navbuilder_tiled — no NavBuilder found; set CIMMERIA_NAVBUILDER");
        return;
    };
    let dir = unique_tempdir("cimmeria-navtiled");
    let chunks = dir.join("chunks");
    std::fs::create_dir_all(&chunks).unwrap();
    write_fixture(&chunks);

    let (code, log, bytes) = run(
        &exe,
        &chunks,
        &dir.join("t1.nav"),
        &["tile=32", "threads=1"],
    );
    if code == 1 && log.contains("Unknown parameter 'tile'") {
        eprintln!(
            "SKIPPED navbuilder_tiled — {} predates tile=; set CIMMERIA_NAVBUILDER to a tree build",
            exe.display()
        );
        return;
    }
    assert_eq!(code, 0, "tiled build failed:\n{log}");
    let bytes = bytes.expect("exit 0 must leave a .nav");
    assert_eq!(&bytes[..4], b"XRCT", "tile= must write the tiled layout");

    let (code, log4, bytes4) = run(
        &exe,
        &chunks,
        &dir.join("t4.nav"),
        &["tile=32", "threads=4"],
    );
    assert_eq!(code, 0, "{log4}");
    assert!(
        bytes4.as_deref() == Some(&bytes[..]),
        "the tiled output must not depend on the worker count"
    );

    let NavFile::Tiled(nav) = NavFile::from_bytes(&bytes).expect("parse tiled nav") else {
        panic!("XRCT file parsed as single-mesh");
    };
    assert!(nav.tiles.len() > 4, "only {} tiles", nav.tiles.len());
    assert!((nav.tile_width - 9.6).abs() < 1e-4, "{}", nav.tile_width);

    let graph = NavGraph::from_tiled(&nav);
    assert_eq!(
        graph.component_count,
        1,
        "the L + ramp is one surface; {} tiles came back as {} components",
        nav.tiles.len(),
        graph.component_count
    );
    for p in [
        [305.0, 50.0, 1005.0],
        [333.0, 50.0, 1010.0],
        [310.0, 53.0, 1070.0],
    ] {
        let hit = graph.locate(p).unwrap();
        assert_eq!(hit.horizontal_distance, 0.0, "{p:?} off the tiled mesh");
    }

    // The seam-straddling platform went, as it would have in a single
    // mesh: with the seam filter reverted it is a second component here.
    let platform = graph.locate([348.0, 50.0, 1041.5]).unwrap();
    assert!(
        platform.horizontal_distance > 3.0,
        "the 3 m platform survived the tiled build ({} m from a polygon)",
        platform.horizontal_distance
    );
    assert!(log.contains("Seam filter:"), "{log}");

    // And it is the seam filter that removed it: with the filter off the
    // tiles hand back the platform as its own island.
    let (code, log0, raw) = run(
        &exe,
        &chunks,
        &dir.join("t0.nav"),
        &["tile=32", "seamFilter=0"],
    );
    assert_eq!(code, 0, "{log0}");
    let NavFile::Tiled(raw) = NavFile::from_bytes(&raw.unwrap()).unwrap() else {
        panic!("XRCT file parsed as single-mesh");
    };
    let raw = NavGraph::from_tiled(&raw);
    assert_eq!(raw.component_count, 2, "{log0}");
    let kept = raw.locate([348.0, 50.0, 1041.5]).unwrap();
    assert_eq!(kept.horizontal_distance, 0.0);
}

#[test]
fn a_tile_size_out_of_range_is_a_usage_error() {
    let Some(exe) = navbuilder_path() else {
        return;
    };
    let dir = unique_tempdir("cimmeria-navtiled-usage");
    let chunks = dir.join("chunks");
    std::fs::create_dir_all(&chunks).unwrap();
    write_fixture(&chunks);
    let (code, log, bytes) = run(&exe, &chunks, &dir.join("x.nav"), &["tile=8"]);
    if log.contains("Unknown parameter 'tile'") {
        return;
    }
    assert_eq!(code, 1, "{log}");
    assert!(bytes.is_none());
}

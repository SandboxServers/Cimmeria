//! The `extract_map` binary, driven over synthetic packages.
//!
//! Exit codes are part of this binary's contract — `tools/build-navmesh.*`
//! and CI both branch on them — and nothing else in the suite runs
//! `main`. These drive it through `CARGO_BIN_EXE_extract_map` against a
//! cooked tree built by [`cimmeria_navmesh_extractor::test_support`], so
//! they run on a runner with no client assets.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use cimmeria_navmesh_extractor::test_support::{
    mesh_package, scratch_dir, ChunkFixture, ModelPayload, StaticMeshPayload, TerrainPayload,
};

fn extract_map_bin() -> &'static str {
    env!("CARGO_BIN_EXE_extract_map")
}

fn run(args: &[&str]) -> Output {
    Command::new(extract_map_bin())
        .args(args)
        .output()
        .expect("spawn extract_map")
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("process exited with a code")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

/// A `CookedPC`-shaped tree with one `Maps/Synth` map: a terrain-plus-BSP
/// chunk and a resolvable `StaticMeshActor`.
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

    let mut chunk = ChunkFixture::new();
    chunk.add_terrain(&TerrainPayload::flat(2, 2));
    chunk.add_level_model(&ModelPayload::horizontal_quad(
        0.0,
        100.0,
        0.0,
        100.0,
        300.0,
        0,
        [0.0, 0.0, 1.0],
    ));
    chunk.add_static_mesh_actor("Floor_0", [0.0, 0.0, 0.0], 1.0, ("Fx-Props", "Fx-Floor01"));
    chunk.write(&maps, "Synth", 0x0000_0001);
    root
}

fn extract_into(tag: &str) -> (PathBuf, PathBuf) {
    let root = cooked_root(&format!("{tag}-cooked"));
    let out = scratch_dir(&format!("{tag}-out"));
    let index = out.join("index.bincode");
    let result = run(&[
        "extract",
        "--cooked-root",
        root.to_str().unwrap(),
        "--map",
        "Synth",
        "--out",
        out.to_str().unwrap(),
        "--index",
        index.to_str().unwrap(),
    ]);
    assert_eq!(code(&result), 0, "stderr: {}", stderr(&result));
    (out, index)
}

#[test]
fn no_arguments_prints_usage_and_exits_zero() {
    let out = run(&[]);
    assert_eq!(code(&out), 0);
    assert!(
        stdout(&out).starts_with("extract_map —"),
        "{}",
        stdout(&out)
    );
    assert!(stdout(&out).contains("USAGE:"));
}

#[test]
fn help_prints_usage_and_exits_zero() {
    for flag in ["-h", "--help", "help"] {
        let out = run(&[flag]);
        assert_eq!(code(&out), 0, "{flag}");
        assert!(stdout(&out).contains("USAGE:"), "{flag}");
    }
}

#[test]
fn a_missing_required_flag_exits_two_with_the_usage_on_stderr() {
    // Exit 2 is "you called me wrong" and is distinct from exit 1,
    // "I ran and failed" — the wrappers branch on the difference.
    let out = run(&["extract", "--map", "Synth"]);
    assert_eq!(code(&out), 2, "stderr: {}", stderr(&out));
    let err = stderr(&out);
    assert!(err.starts_with("error: "), "{err}");
    assert!(err.contains("--cooked-root"), "{err}");
    assert!(err.contains("USAGE:"), "usage must follow the error: {err}");
}

#[test]
fn an_unknown_flag_exits_two() {
    let out = run(&["probe", "--obj-dir", ".", "--not-a-flag", "x"]);
    assert_eq!(code(&out), 2, "stderr: {}", stderr(&out));
}

#[test]
fn a_missing_map_directory_exits_one() {
    let root = scratch_dir("cli-missing-map");
    let out_dir = scratch_dir("cli-missing-map-out");
    let out = run(&[
        "extract",
        "--cooked-root",
        root.to_str().unwrap(),
        "--map",
        "NoSuchMap",
        "--out",
        out_dir.to_str().unwrap(),
        "--index",
        out_dir.join("i.bin").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1, "stdout: {}", stdout(&out));
    assert!(
        stderr(&out).contains("map directory not found"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn extract_writes_the_objs_both_reports_and_the_index_cache() {
    let (out_dir, index) = extract_into("cli-extract");

    assert!(out_dir.join("00000001o.obj").is_file());
    assert!(index.is_file(), "the index cache is built and saved");

    let coverage = std::fs::read_to_string(out_dir.join("coverage.tsv")).expect("coverage.tsv");
    let header = coverage.lines().next().unwrap();
    assert!(header.starts_with("chunk\tchunk_id"), "{header}");
    let row = coverage
        .lines()
        .find(|l| l.starts_with("Synth-00000001\t"))
        .expect("a row for the chunk");
    let cols: Vec<&str> = row.split('\t').collect();
    let col = |name: &str| {
        header
            .split('\t')
            .position(|h| h == name)
            .unwrap_or_else(|| panic!("no {name} column"))
    };
    assert_eq!(cols[col("terrain_triangles")], "8");
    assert_eq!(cols[col("bsp_triangles")], "2");
    assert_eq!(cols[col("staticmesh_triangles")], "1");
    assert_eq!(cols[col("sources_balanced")], "yes");

    let classes =
        std::fs::read_to_string(out_dir.join("coverage_classes.tsv")).expect("classes tsv");
    assert!(
        classes.lines().any(|l| l.starts_with("Terrain\t")),
        "{classes}"
    );
}

#[test]
fn extract_reloads_a_cached_index_on_the_second_run() {
    let (out_dir, index) = extract_into("cli-index-cache");
    let before = std::fs::metadata(&index).unwrap().len();

    // Re-run against the same cache: the binary must say it loaded
    // rather than built, and must not rewrite the file.
    let cooked = cooked_root("cli-index-cache-second");
    let second = run(&[
        "extract",
        "--cooked-root",
        cooked.to_str().unwrap(),
        "--map",
        "Synth",
        "--out",
        out_dir.to_str().unwrap(),
        "--index",
        index.to_str().unwrap(),
    ]);
    assert_eq!(code(&second), 0, "stderr: {}", stderr(&second));
    assert!(
        stderr(&second).contains("loaded package index"),
        "{}",
        stderr(&second)
    );
    assert_eq!(std::fs::metadata(&index).unwrap().len(), before);
}

#[test]
fn extract_refuses_a_combined_obj_inside_the_output_directory_and_exits_one() {
    let root = cooked_root("cli-combined-cooked");
    let out_dir = scratch_dir("cli-combined-out");
    let out = run(&[
        "extract",
        "--cooked-root",
        root.to_str().unwrap(),
        "--map",
        "Synth",
        "--out",
        out_dir.to_str().unwrap(),
        "--index",
        out_dir.join("i.bin").to_str().unwrap(),
        "--combined",
        out_dir.join("whole.obj").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1, "stdout: {}", stdout(&out));
    assert!(
        stderr(&out).contains("per-chunk output directory"),
        "{}",
        stderr(&out)
    );
    assert!(!out_dir.join("whole.obj").exists());
}

/// A points file naming one spot above the synthetic terrain and one
/// far away from any geometry, in BigWorld units under the CA05
/// mapping (`bw = (ue.Y, ue.Z, ue.X) / 100`).
fn points_file(dir: &Path) -> PathBuf {
    // Terrain spans UE3 x,y in 0..200 at z = 0, so under CA05 it covers
    // bw x 0..2, bw z 0..2 at bw y = 0.
    let path = dir.join("points.tsv");
    std::fs::write(
        &path,
        "label\tconfidence\tx\ty\tz\tsource\r\n\
         OnTheFloor\tHIGH\t1.0\t0.4\t1.0\tsynthetic terrain\r\n\
         MilesAway\tHIGH\t900.0\t0.4\t900.0\tnowhere near the fixture\r\n",
    )
    .unwrap();
    path
}

#[test]
fn probe_scores_a_point_over_the_synthetic_floor_and_misses_one_off_the_map() {
    let (obj_dir, _index) = extract_into("cli-probe");
    let points = points_file(&obj_dir);

    let out = run(&[
        "probe",
        "--obj-dir",
        obj_dir.to_str().unwrap(),
        "--mapping",
        "+Y+Z+X",
        "--points",
        points.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));

    let summary = std::fs::read_to_string(obj_dir.join("probe_mappings.tsv")).expect("summary");
    let header = summary.lines().next().unwrap();
    assert!(header.starts_with("mapping\thigh_floor_hits"), "{header}");
    let row: Vec<&str> = summary
        .lines()
        .nth(1)
        .expect("one mapping row")
        .split('\t')
        .collect();
    assert_eq!(row[0], "+Y+Z+X");
    assert_eq!(row[2], "2", "both points are HIGH confidence");
    assert_eq!(
        row[1], "1",
        "exactly the point over the terrain finds a floor: {summary}"
    );

    let detail = std::fs::read_to_string(obj_dir.join("probe_points.tsv")).expect("detail");
    assert!(detail.contains("OnTheFloor"), "{detail}");
    assert!(detail.contains("MilesAway"), "{detail}");
}

#[test]
fn probe_over_an_empty_directory_exits_one() {
    let empty = scratch_dir("cli-probe-empty");
    let out = run(&["probe", "--obj-dir", empty.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "stdout: {}", stdout(&out));
    assert!(
        stderr(&out).contains("run `extract` first"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn probe_rejects_a_malformed_points_file_and_exits_one() {
    let (obj_dir, _index) = extract_into("cli-probe-badpoints");
    let bad = obj_dir.join("bad.tsv");
    std::fs::write(&bad, "OnlyThree\tHIGH\t1.0\r\n").unwrap();
    let out = run(&[
        "probe",
        "--obj-dir",
        obj_dir.to_str().unwrap(),
        "--points",
        bad.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1, "stdout: {}", stdout(&out));
    assert!(stderr(&out).contains("tab-separated"), "{}", stderr(&out));
}

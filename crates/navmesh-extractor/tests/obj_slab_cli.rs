//! End-to-end tests for the `obj_slab` binary.
//!
//! No client tree and no NavBuilder: the chunk OBJs are written by
//! [`cimmeria_navmesh_extractor::obj::write_obj`], the same emitter
//! `extract_map` uses, so the process reads exactly the artifact shape
//! it reads in production. The point here is the process contract —
//! exit codes and the lines an operator greps for — which the in-binary
//! unit tests cannot cover.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_navmesh_extractor::obj::write_obj;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cimmeria-obj-slab-cli-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// BigWorld metres → UE3 centimetres: `ue = (bw.z, bw.x, bw.y) * 100`.
fn ue_cm(bw: [f32; 3]) -> [f32; 3] {
    [bw[2] * 100.0, bw[0] * 100.0, bw[1] * 100.0]
}

/// A floor quad wound the way Recast reads as walkable.
fn floor(x0: f32, z0: f32, x1: f32, z1: f32, y: f32) -> [[[f32; 3]; 3]; 2] {
    [
        [ue_cm([x0, y, z0]), ue_cm([x1, y, z0]), ue_cm([x0, y, z1])],
        [ue_cm([x1, y, z0]), ue_cm([x1, y, z1]), ue_cm([x0, y, z1])],
    ]
}

fn write_chunk(dir: &Path, stem: &str, tris: &[[[f32; 3]; 3]]) {
    let mut soup = TriangleSoup::new(Some(format!("Chunk_{stem}")));
    for t in tris {
        soup.push(*t);
    }
    write_obj(&dir.join(format!("{stem}.obj")), &soup).unwrap();
}

fn obj_slab(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_obj_slab"))
        .args(args)
        .output()
        .expect("obj_slab must be built by `cargo test`")
}

fn code_of(out: &Output) -> i32 {
    out.status.code().expect("no signal on Windows")
}

#[test]
fn a_box_with_geometry_reports_it_and_exits_zero() {
    let dir = scratch("ok");
    write_chunk(&dir, "00000000o", &floor(10.0, 10.0, 30.0, 30.0, 4.0));

    let out = obj_slab(&[
        dir.to_str().unwrap(),
        "--box",
        "room=9,3,9,31,6,31",
        "--column",
        "20,20",
    ]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(code_of(&out), 0, "stdout:\n{text}");
    assert!(text.contains("chunks     1 read, 0 skipped"), "{text}");
    assert!(text.contains("box room"), "{text}");
    assert!(text.contains("triangles  2"), "{text}");
    assert!(text.contains("flat<=5deg 400.0 m^2"), "{text}");
    assert!(
        text.contains("y=    4.00  tilt=  0.0 deg  faces_up=true"),
        "the floor must read as a floor, not a ceiling:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_empty_box_exits_two() {
    let dir = scratch("empty");
    write_chunk(&dir, "00000000o", &floor(10.0, 10.0, 30.0, 30.0, 4.0));
    let out = obj_slab(&[dir.to_str().unwrap(), "--at", "void=50,4,50,2,2"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(code_of(&out), 2, "stdout:\n{text}");
    assert!(text.contains("EMPTY"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_arguments_a_missing_directory_and_a_corrupt_obj_all_exit_one() {
    let dir = scratch("usage");
    assert_eq!(code_of(&obj_slab(&[])), 1, "no directory, no box");

    let out = obj_slab(&[dir.to_str().unwrap(), "--not-a-flag"]);
    assert_eq!(code_of(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown flag"));

    // `--cell 0` used to divide through to an infinite grid width and
    // allocate until the process died; `inf` poisoned every bound.
    for bad in [["--cell", "0"], ["--tilt", "inf"], ["--margin", "-3"]] {
        let out = obj_slab(&[dir.to_str().unwrap(), "--at", "g=0,0,0", bad[0], bad[1]]);
        assert_eq!(code_of(&out), 1, "{bad:?} must be refused");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(bad[0]),
            "the error must name the flag: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let missing = dir.join("no-such-dir");
    let out = obj_slab(&[missing.to_str().unwrap(), "--at", "g=0,0,0"]);
    assert_eq!(code_of(&out), 1, "missing directory");
    assert!(!String::from_utf8_lossy(&out.stderr).is_empty());

    // A malformed `v` line renumbers every later face index, so the tool
    // must refuse rather than measure geometry that is not there.
    std::fs::write(dir.join("00000000o.obj"), "v 0 0 0\r\nv 1 junk 1\r\n").unwrap();
    let out = obj_slab(&[dir.to_str().unwrap(), "--at", "g=0,0,0,500,500"]);
    assert_eq!(code_of(&out), 1, "corrupt OBJ");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("malformed vertex"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

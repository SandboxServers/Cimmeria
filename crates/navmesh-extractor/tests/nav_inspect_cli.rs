//! End-to-end tests for the `nav_inspect` binary.
//!
//! These need no client tree and no NavBuilder: the `.nav` files are written
//! by [`XrcNav::write`], which is the same serialiser the round-trip test
//! pins against the shipped 2013 `castle_cellblock.nav`. The point is the
//! process contract — exit codes and the lines a CI gate or a human greps
//! for — which the in-binary unit tests cannot cover.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use cimmeria_navmesh_extractor::nav_roundtrip::XrcNav;

/// `n` unlinked one-metre quads spaced `spacing` metres apart along x, all at
/// height `y`. `cs = ch = 1` and `bmin = 0`, so grid units are world metres.
fn islands(n: u16, spacing: u16, y: u16) -> XrcNav {
    let mut verts: Vec<u16> = Vec::new();
    let mut polys: Vec<u16> = Vec::new();
    for i in 0..n {
        let x = i * spacing;
        for (dx, dz) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
            verts.extend_from_slice(&[x + dx, y, dz]);
        }
        let b = i * 4;
        polys.extend_from_slice(&[b, b + 1, b + 2, b + 3, 0xffff, 0xffff, 0xffff, 0xffff]);
    }
    XrcNav {
        agent_height: 1.8,
        agent_climb: 0.6,
        agent_radius: 0.6,
        nverts: u32::from(n) * 4,
        npolys: u32::from(n),
        nvp: 4,
        border_size: 0,
        cs: 1.0,
        ch: 1.0,
        bmin: [0.0, 0.0, 0.0],
        bmax: [1000.0, 100.0, 1000.0],
        verts,
        polys,
        regs: vec![0; n as usize],
        flags: vec![1; n as usize],
        areas: vec![63; n as usize],
        detail_nmeshes: 0,
        detail_nverts: 0,
        detail_ntris: 0,
        detail_meshes: vec![],
        detail_verts: vec![],
        detail_tris: vec![],
    }
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cimmeria-nav-inspect-cli-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_nav(dir: &Path, name: &str, nav: &XrcNav) -> PathBuf {
    let path = dir.join(name);
    let mut buf: Vec<u8> = Vec::new();
    nav.write(&mut buf).expect("serialise");
    std::fs::write(&path, &buf).unwrap();
    path
}

fn nav_inspect(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nav_inspect"))
        .args(args)
        .output()
        .expect("nav_inspect must be built by `cargo test`")
}

fn code_of(out: &Output) -> i32 {
    out.status.code().expect("no signal on Windows")
}

#[test]
fn all_probes_in_one_component_exit_zero() {
    let dir = scratch("ok");
    let path = write_nav(&dir, "one.nav", &islands(1, 10, 5));
    let out = nav_inspect(&[path.to_str().unwrap(), "--probe", "a=0.5,5,0.5"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(code_of(&out), 0, "stdout:\n{text}");
    assert!(text.contains("components  1"), "{text}");
    assert!(
        text.contains("header      nverts=4 npolys=1 nvp=4"),
        "{text}"
    );
    assert!(
        text.contains("agent       height=1.8 climb=0.6 radius=0.6"),
        "{text}"
    );
    assert!(text.trim_end().ends_with("ok"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_probe_out_of_tolerance_exits_two() {
    let dir = scratch("tol");
    let path = write_nav(&dir, "one.nav", &islands(1, 10, 5));
    let out = nav_inspect(&[path.to_str().unwrap(), "--probe", "far=40,5,0.5", "--quiet"]);
    assert_eq!(code_of(&out), 2);
    assert!(String::from_utf8_lossy(&out.stdout).contains("OUT OF TOLERANCE"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("out of tolerance"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn probes_spanning_components_exit_three_and_gaps_name_the_chain() {
    let dir = scratch("split");
    // Three islands, 1 m between neighbours and 3 m end to end.
    let path = write_nav(&dir, "three.nav", &islands(3, 2, 5));
    let out = nav_inspect(&[
        path.to_str().unwrap(),
        "--probe",
        "a=0.5,5,0.5",
        "--probe",
        "c=4.5,5,0.5",
        "--quiet",
        "--gaps",
        "--gap-h",
        "1.5",
    ]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(code_of(&out), 3, "stdout:\n{text}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("probes span 2 components"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains("chain: 2 hop(s), widest 1.00 m, total 2.00 m"),
        "the chain search must report both 1 m hops:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn max_components_exceeded_exits_four() {
    let dir = scratch("maxc");
    let path = write_nav(&dir, "three.nav", &islands(3, 50, 5));
    let out = nav_inspect(&[path.to_str().unwrap(), "--quiet", "--max-components", "2"]);
    assert_eq!(code_of(&out), 4);
    assert!(String::from_utf8_lossy(&out.stderr).contains("exceeds --max-components 2"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_arguments_and_a_missing_file_both_exit_one() {
    let dir = scratch("usage");
    assert_eq!(code_of(&nav_inspect(&[])), 1, "no file argument");
    let out = nav_inspect(&["--not-a-flag"]);
    assert_eq!(code_of(&out), 1);
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown flag"));

    let missing = dir.join("nope.nav");
    let out = nav_inspect(&[missing.to_str().unwrap()]);
    assert_eq!(code_of(&out), 1, "missing file");
    assert!(!String::from_utf8_lossy(&out.stderr).is_empty());

    // A file that exists but is not a .nav must fail the same way rather
    // than producing a bogus report.
    let junk = dir.join("junk.nav");
    std::fs::write(&junk, b"not a navmesh").unwrap();
    assert_eq!(code_of(&nav_inspect(&[junk.to_str().unwrap()])), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

//! Phase 0 acceptance test: read `data/spaces/castle_cellblock.nav`,
//! re-emit it, and compare byte-for-byte against the source.
//!
//! This proves the XRC `.nav` wire format is fully understood — any
//! future Rust replacement for the C++ NavBuilder can write the same
//! shape and be loaded by `crates/entity/src/navigation.rs::NavMesh::load`
//! without code changes.
//!
//! Self-skips when the fixture is absent so it doesn't fail in CI
//! environments that don't ship the data dir (matches the pattern in
//! `crates/entity/src/navigation.rs` tests).

use std::path::PathBuf;

use cimmeria_navmesh_extractor::nav_roundtrip::XrcNav;

fn fixture_path() -> PathBuf {
    // CARGO_MANIFEST_DIR resolves to crates/navmesh-extractor — climb
    // two levels to reach the repo root, then descend into data/spaces.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("data").join("spaces").join("castle_cellblock.nav"))
        .expect("could not resolve repo-relative path to castle_cellblock.nav")
}

#[test]
fn castle_cellblock_nav_round_trips_byte_exact() {
    let path = fixture_path();
    if !path.exists() {
        eprintln!(
            "Skipping castle_cellblock.nav round-trip — fixture not present at {}",
            path.display()
        );
        return;
    }

    let bytes = std::fs::read(&path).expect("read castle_cellblock.nav");
    let original_size = bytes.len();

    let parsed = XrcNav::round_trip(&bytes).expect("round-trip mismatch");

    // Sanity-check the parsed metadata matches the values quoted in the
    // asset-pipeline deep dive — guards against silent format drift.
    assert_eq!(parsed.agent_height, 0.6);
    assert_eq!(parsed.agent_climb, 0.9);
    assert_eq!(parsed.agent_radius, 0.6);
    assert_eq!(parsed.nverts, 2778);
    assert_eq!(parsed.npolys, 1479);
    assert_eq!(parsed.nvp, 6);
    assert_eq!(parsed.detail_nmeshes, 1479);
    assert_eq!(parsed.detail_nverts, 6031);
    assert_eq!(parsed.detail_ntris, 3102);

    // 0x29 08B from the deep dive — the parser must reach EOF exactly.
    // The XrcNav::read() check ensures no trailing bytes; reasserting
    // here makes the "the format is the format" claim visible.
    assert_eq!(original_size, 0x29_08B);
}

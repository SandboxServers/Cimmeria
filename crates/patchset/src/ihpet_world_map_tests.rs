//! `013-ihpet-world-map`: rebuilds the `world__default_` overview texture in
//! the stock `Ihpet_Crater_Light_MapData.upk` (see `data/client-patches/README.md`).
//! These tests read the committed zip only; the last one, ignored, applies it to
//! a real client file.

use std::path::Path;

use crate::debug_area_rings_tests::{bsdiff_blocks, entry, recipe};
use crate::recipe::Transform;

const ID: &str = "013-ihpet-world-map";
const MAPDATA: &str =
    "Working/SGWGame/CookedPC/Maps/Ihpet_Crater_Light/Ihpet_Crater_Light_MapData.upk";
/// The 2009 file's SHA-256, the one `tools/client-patches/ihpet_world_map.py` accepts.
const STOCK_SHA256: &str = "ea86f7c32b6d8e230c86191b5f048e080fc979e80d584bbdc878e3db5404a1f4";

/// One op, one file, one start: the stock MapData file as it ships, with no
/// transform and no other patch's output. 001-012 never write this file, so
/// the patch has no ordering constraint; a source that named another patch's
/// output would give it one.
#[test]
fn committed_013_rebuilds_the_stock_map_data_file_and_needs_no_other_patch() {
    let (r, z) = recipe(ID);
    assert_eq!(r.ops.len(), 1, "{r:?}");
    let op = &r.ops[0];
    assert_eq!(op.target, MAPDATA);
    assert_eq!(op.sources.len(), 1, "{op:?}");
    assert_eq!(op.sources[0].path, MAPDATA);
    assert_eq!(op.sources[0].transform, Transform::None);
    assert_eq!(op.sources[0].output_of, None);
    assert_eq!(op.sources[0].sha256, STOCK_SHA256);
    assert!(op.alternatives.is_empty());
    assert_ne!(
        op.result_sha256, STOCK_SHA256,
        "the patch must change the file"
    );
    // The recipe and its one delta.
    assert_eq!(z.len(), 2);
}

/// The delta may carry the new texture (about 400 KB of compressed picture
/// data, which the README calls out as a maintainer call) and nothing like the
/// stock package. A zip that embedded the stock file's 6 MB of texture tiles,
/// or a delta against nothing, would blow these bounds.
#[test]
fn committed_013_delta_does_not_carry_the_stock_package() {
    let (r, mut z) = recipe(ID);
    let delta = entry(&mut z, &r.ops[0].delta);
    let (_, _, extra, new_size) = bsdiff_blocks(&delta);
    assert!(
        new_size > 6_000_000,
        "result is {new_size} bytes, the stock file is 6,167,790"
    );
    assert!(
        extra < 450_000,
        "extra block is {extra} compressed bytes: more than the one rebuilt texture"
    );
    assert!(delta.len() < 450_000, "delta is {} bytes", delta.len());
}

/// Nothing in CI can regenerate the file (it is derived from CME's map data,
/// which the repo does not hold), so the committed bytes are pinned to what the
/// README tells the coordinator to publish: the zip's SHA-256 and size, the
/// result hash and the stock hash the generator accepts.
#[test]
fn the_readme_states_the_committed_013_zip_and_result_hashes() {
    use sha2::{Digest, Sha256};
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/client-patches");
    let readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
    let zip = std::fs::read(dir.join(format!("{ID}.zip"))).unwrap();
    let zip_sha: String = Sha256::digest(&zip)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let (r, _) = recipe(ID);
    let generator = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/client-patches/ihpet_world_map.py"),
    )
    .unwrap();
    // The README writes sizes with thousands separators.
    let size = zip.len();
    let size = format!("{},{:03}", size / 1000, size % 1000);
    for (what, value, doc) in [
        ("013 zip sha256", zip_sha, &readme),
        ("013 zip size", size, &readme),
        ("013 result sha256", r.ops[0].result_sha256.clone(), &readme),
        ("stock MapData sha256", STOCK_SHA256.to_string(), &readme),
        ("stock MapData sha256", STOCK_SHA256.to_string(), &generator),
    ] {
        assert!(
            doc.contains(&value),
            "the README or generator does not state the {what} ({value}); \
             update it with the zip, or the zip is not the one it describes"
        );
    }
}

// Manual check against a real client: SGW_STOCK_MAPDATA = a stock
// Ihpet_Crater_Light_MapData.upk. 013 is applied to a copy and must end on the
// recipe's result hash, and a second apply must find it current.
//   cargo test -p cimmeria-patchset real_client_ihpet_world_map -- --ignored --nocapture
#[test]
#[ignore = "needs the stock Ihpet_Crater_Light_MapData.upk; see the comment"]
fn real_client_ihpet_world_map() {
    let stock = std::path::PathBuf::from(std::env::var("SGW_STOCK_MAPDATA").unwrap());
    let zip = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/client-patches/013-ihpet-world-map.zip");
    let (r, _) = recipe(ID);
    let tree = tempfile::tempdir().unwrap();
    let to = tree.path().join(MAPDATA);
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(&stock, &to).unwrap();
    let report = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(report.rebuilt, vec![MAPDATA.to_string()]);
    let rebuilt = std::fs::read(&to).unwrap();
    assert_eq!(crate::sha256_hex(&rebuilt), r.ops[0].result_sha256);
    let again = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(again.already_current, vec![MAPDATA.to_string()]);
}

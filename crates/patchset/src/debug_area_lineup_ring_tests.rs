//! `014-debug-area-lineup-ring`: a ninth Debug Area ring rig, for the Lineup
//! station (region 43, DA-11), cloned into the Ihpet chunk that
//! `011-debug-area-rings-fix` writes. The rig is the same Castle region 3 rig
//! 011's eight are, but the delta needs only 011's chunk: every byte the new
//! rig adds already sits in one of 011's copies. These tests read the
//! committed zips only; the last one, ignored, applies 014 to a real client.

use std::path::Path;

use crate::debug_area_rings_tests::{
    bsdiff_blocks, entry, recipe, rig_positions, seeded_rigs, IHPET,
};
use crate::recipe::Transform;

const ID: &str = "014-debug-area-lineup-ring";
const RINGS_FIX: &str = "011-debug-area-rings-fix";
/// The Teleport Out sequence of the rig 014 adds (the seed's region 43).
const LINEUP_RIG: &str =
    "Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_7";

/// 014 starts from 011's output and nothing else: not the stock chunk, not
/// 010's broken chunk, not the Armory map. Every other state is 011's job,
/// so a launcher that skips 011 skips 014 too, and a 014 that fails leaves
/// the chunk 011 wrote.
#[test]
fn committed_014_starts_from_011_output_only() {
    let (r014, z014) = recipe(ID);
    let (r011, _) = recipe(RINGS_FIX);
    assert_eq!(r014.ops.len(), 1, "{r014:?}");
    let op = &r014.ops[0];
    assert_eq!(op.target, IHPET);
    assert_eq!(op.sources.len(), 1, "{op:?}");
    let source = &op.sources[0];
    assert_eq!(source.path, IHPET);
    assert_eq!(
        source.transform,
        Transform::None,
        "011's output is already normalized"
    );
    assert_eq!(
        source.sha256, r011.ops[0].result_sha256,
        "014 must start from the exact chunk 011 writes"
    );
    assert_eq!(source.output_of.as_deref(), Some(RINGS_FIX));
    assert!(op.alternatives.is_empty(), "{op:?}");
    assert_ne!(op.result_sha256, r011.ops[0].result_sha256);
    // The recipe and the one delta: no whole files.
    assert_eq!(z014.len(), 2);
}

/// The no-CME-bytes rule. One more copy of a rig the chunk already holds
/// eight of is "copy these bytes again", so the delta is tiny and its extra
/// block (the only bytes that reach the map verbatim) is 14 compressed bytes:
/// the new coordinates and indices.
#[test]
fn committed_014_delta_ships_no_verbatim_map_bytes() {
    let (r014, mut z014) = recipe(ID);
    let delta = entry(&mut z014, &r014.ops[0].delta);
    let (_, _, extra, new_size) = bsdiff_blocks(&delta);
    assert!(extra <= 64, "extra block is {extra} compressed bytes");
    assert!(delta.len() < 2 * 1024, "delta is {} bytes", delta.len());
    assert!(new_size > 2_000_000, "result is {new_size} bytes");
}

/// The seed's Lineup station plays the rig 014 adds, and no other station
/// does: 011's chunk has `_Seq` to `_Seq_6`, so the ninth copy takes `_Seq_7`.
#[test]
fn the_seed_gives_the_lineup_rig_to_one_station() {
    let lineup: Vec<_> = seeded_rigs()
        .into_iter()
        .filter(|r| r.0 == LINEUP_RIG)
        .collect();
    assert_eq!(lineup.len(), 1, "{lineup:?}");
    let (_, x, z) = &lineup[0];
    assert!((x - 287.0).abs() < 0.01 && (z + 914.0).abs() < 0.01);
}

/// Nothing in CI can rebuild the chunk, so the committed bytes are pinned to
/// what the README tells the coordinator to publish (see
/// `the_readme_states_the_committed_011_zip_and_result_hashes`).
#[test]
fn the_readme_states_the_committed_014_zip_and_result_hashes() {
    use sha2::{Digest, Sha256};
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/client-patches");
    let readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
    let zip = std::fs::read(dir.join(format!("{ID}.zip"))).unwrap();
    let zip_sha: String = Sha256::digest(&zip)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let (r014, _) = recipe(ID);
    for (what, value) in [
        ("014 zip sha256", zip_sha),
        ("014 zip size", zip.len().to_string()),
        ("014 result sha256", r014.ops[0].result_sha256.clone()),
    ] {
        assert!(
            readme.contains(&value),
            "data/client-patches/README.md does not state the {what} ({value}); \
             update it with the zip, or the zip is not the one the README describes"
        );
    }
}

// Manual check against a real client: SGW_011_CHUNK = a file with 011's
// output (sha256 52b4f3ad...; a launcher install with 011 applied holds it at
// Working/SGWGame/CookedPC/Maps/Ihpet_Crater_Light/). 014 is applied to a copy,
// and the result must match the recipe, load on the client (name audit) and
// hold all nine rigs where the seed puts the pads.
//   cargo test -p cimmeria-patchset real_client_debug_area_lineup_ring -- --ignored --nocapture
#[test]
#[ignore = "needs 011's output chunk; see the comment"]
fn real_client_debug_area_lineup_ring() {
    let chunk = std::path::PathBuf::from(std::env::var("SGW_011_CHUNK").unwrap());
    let zip = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/client-patches")
        .join(format!("{ID}.zip"));
    let (r014, _) = recipe(ID);

    let tree = tempfile::tempdir().unwrap();
    let to = tree.path().join(IHPET);
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(&chunk, &to).unwrap();
    let report = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(report.rebuilt, vec![IHPET.to_string()]);
    let rebuilt = std::fs::read(&to).unwrap();
    assert_eq!(crate::sha256_hex(&rebuilt), r014.ops[0].result_sha256);
    let again = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(again.already_current, vec![IHPET.to_string()]);

    // 010's lesson: every property name of every client-loaded object must
    // load on the client.
    let package = cimmeria_upk::Package::open(&to).unwrap();
    let audit =
        cimmeria_upk::patcher::name_audit::audit_client_names(&package, 0..package.exports.len())
            .unwrap();
    assert_eq!(audit.unloadable, vec![]);
    assert!(audit.not_audited.is_empty(), "{:?}", audit.not_audited);
    assert!(audit.audited > 800, "audited {}", audit.audited);

    let built = rig_positions(&to);
    assert_eq!(built.len(), 9, "{built:?}");
    for (path, x, z) in seeded_rigs() {
        let (_, bx, bz) = built
            .iter()
            .find(|b| b.0 == path)
            .unwrap_or_else(|| panic!("{path} is not in the rebuilt map: {built:?}"));
        assert!(
            (bx - x).abs() < 0.05 && (bz - z).abs() < 0.05,
            "{path}: rig at ({bx}, {bz}), seeded pad at ({x}, {z})"
        );
    }
}

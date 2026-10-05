//! `011-debug-area-rings-fix`: replaces `010-debug-area-rings`, which hung the
//! client (see `data/client-patches/README.md`). One op, two starting
//! points: clean installs rebuild the Ihpet chunk from the normalized stock
//! chunk plus 007's Armory map, installs that applied 010 rebuild it from
//! 010's own output. These tests read the committed zips only; the last one,
//! ignored, applies 011 to a real client both ways.

use std::path::Path;

use crate::debug_area_rings_tests::{
    bsdiff_blocks, entry, recipe, rig_positions, seeded_rigs, ARMORY, IHPET,
};
use crate::recipe::Transform;

const ID: &str = "011-debug-area-rings-fix";

/// The primary start is 010's start (stock chunk + the Armory map as 007
/// leaves it); the alternative is 010's output alone; both reach one hash.
#[test]
fn committed_011_has_a_clean_install_start_and_a_repair_start_for_010() {
    let (r011, z011) = recipe(ID);
    let (r010, _) = recipe("010-debug-area-rings");
    assert_eq!(r011.ops.len(), 1, "{r011:?}");
    let (op, op010) = (&r011.ops[0], &r010.ops[0]);
    assert_eq!(op.target, IHPET);

    // Clean installs: exactly 010's sources, so the same installs qualify.
    assert_eq!(op.sources, op010.sources);
    assert_eq!(op.sources[0].transform, Transform::UpkNormalize);
    assert_eq!(op.sources[1].path, ARMORY);
    assert_eq!(
        op.sources[1].output_of.as_deref(),
        Some("007-castle-armory-ring")
    );

    // Installs that applied 010: its output, byte for byte, nothing else. A
    // transform here would normalize a file that is already normalized.
    assert_eq!(op.alternatives.len(), 1, "{op:?}");
    let alt = &op.alternatives[0];
    assert_eq!(alt.sources.len(), 1);
    assert_eq!(alt.sources[0].path, IHPET);
    assert_eq!(alt.sources[0].transform, Transform::None);
    assert_eq!(
        alt.sources[0].sha256, op010.result_sha256,
        "the repair must start from the exact file 010 wrote"
    );
    assert_eq!(
        alt.sources[0].output_of.as_deref(),
        Some("010-debug-area-rings")
    );

    // 011 never rebuilds 010's file into itself: that would be a no-op that
    // leaves the crash in place.
    assert_ne!(op.result_sha256, op010.result_sha256);
    assert_ne!(op.delta, alt.delta);
    // The recipe and its two deltas, no whole files.
    assert_eq!(z011.len(), 3);
}

/// The no-CME-bytes rule, as `committed_010_delta_ships_no_verbatim_map_bytes`
/// states it: 010's extra block was 487 compressed bytes, and the fix adds
/// none (the same rig data, plus one name entry). The repair start's delta
/// is mostly "keep the old bytes": 14 bytes of extra and 402 in all.
#[test]
fn committed_011_deltas_ship_no_verbatim_map_bytes() {
    let (r011, mut z011) = recipe(ID);
    let op = &r011.ops[0];
    let primary = entry(&mut z011, &op.delta);
    let (_, _, extra, new_size) = bsdiff_blocks(&primary);
    assert!(
        extra <= 520,
        "primary extra block is {extra} compressed bytes"
    );
    assert!(
        primary.len() < 32 * 1024,
        "delta is {} bytes",
        primary.len()
    );
    assert!(new_size > 2_000_000, "result is {new_size} bytes");

    let repair = entry(&mut z011, &op.alternatives[0].delta);
    let (_, _, extra, repaired) = bsdiff_blocks(&repair);
    assert!(
        extra <= 64,
        "repair extra block is {extra} compressed bytes"
    );
    assert!(
        repair.len() < 2 * 1024,
        "repair delta is {} bytes",
        repair.len()
    );
    assert_eq!(repaired, new_size, "both starts must rebuild one file");
}

/// The README and the launcher both promise that fresh installs never see
/// 010. That is a manifest matter (010 is removed from it), but the specs
/// must not give 011 a way to need 010: the clean start has to stand alone.
#[test]
fn the_clean_install_start_does_not_name_010() {
    let (r011, _) = recipe(ID);
    let op = &r011.ops[0];
    assert!(op
        .sources
        .iter()
        .all(|s| s.output_of.as_deref() != Some("010-debug-area-rings")));
}

/// Nothing in CI can rebuild the chunk (it is derived from CME's map, which the
/// repo does not hold), so the committed bytes are pinned to what the README
/// tells the coordinator to publish: the zip's SHA-256 and size and the chunk's
/// result hash. A zip rebuilt with an older cloner and re-pinned in its own
/// recipe still passes the recipe tests above, but its hashes then disagree
/// with the README, so someone has to change both on purpose. The chunk itself
/// is checked by `real_client_debug_area_rings_fix` (name audit and rig
/// positions), which needs the real client files.
#[test]
fn the_readme_states_the_committed_011_zip_and_result_hashes() {
    use sha2::{Digest, Sha256};
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/client-patches");
    let readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
    let zip = std::fs::read(dir.join(format!("{ID}.zip"))).unwrap();
    let zip_sha: String = Sha256::digest(&zip)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let (r011, _) = recipe(ID);
    for (what, value) in [
        ("011 zip sha256", zip_sha),
        ("011 zip size", zip.len().to_string()),
        ("011 result sha256", r011.ops[0].result_sha256.clone()),
        (
            "010 result sha256 (the repair's start)",
            recipe("010-debug-area-rings").0.ops[0]
                .result_sha256
                .clone(),
        ),
    ] {
        assert!(
            readme.contains(&value),
            "data/client-patches/README.md does not state the {what} ({value}); \
             update it with the zip, or the zip is not the one the README describes"
        );
    }
}

// Manual check against a real client: SGW_PATCHED_CLIENT = a client that has
// 007 applied (the QA client does) and a stock Ihpet_Crater_Light-fff80002;
// SGW_010_CHUNK = a file with 010's output (sha256 62ef4acd...). 011 is
// applied to a clean copy and to a copy holding 010's chunk, and both must
// end on the recipe's result hash with every seeded rig where the seed says.
//   cargo test -p cimmeria-patchset real_client_debug_area_rings_fix -- --ignored --nocapture
#[test]
#[ignore = "needs a client with 007 applied and 010's output; see the comment"]
fn real_client_debug_area_rings_fix() {
    let client = std::path::PathBuf::from(std::env::var("SGW_PATCHED_CLIENT").unwrap());
    let broken = std::path::PathBuf::from(std::env::var("SGW_010_CHUNK").unwrap());
    let zip = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/client-patches/011-debug-area-rings-fix.zip");
    let (r011, _) = recipe(ID);
    let want = &r011.ops[0].result_sha256;

    for start in ["clean", "010-installed"] {
        let tree = tempfile::tempdir().unwrap();
        for rel in [IHPET, ARMORY] {
            let to = tree.path().join(rel);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(client.join(rel), &to).unwrap();
        }
        if start == "010-installed" {
            std::fs::copy(&broken, tree.path().join(IHPET)).unwrap();
            // The repair does not read the Armory map.
            std::fs::remove_file(tree.path().join(ARMORY)).unwrap();
        }
        let report = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
        assert_eq!(report.rebuilt, vec![IHPET.to_string()], "{start}");
        let rebuilt = std::fs::read(tree.path().join(IHPET)).unwrap();
        assert_eq!(&crate::sha256_hex(&rebuilt), want, "{start}");
        let again = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
        assert_eq!(again.already_current, vec![IHPET.to_string()], "{start}");

        // Every property name in the rebuilt chunk's client-loaded objects must
        // load on the client: 010 had 48 that did not, and froze the client.
        let package = cimmeria_upk::Package::open(tree.path().join(IHPET)).unwrap();
        let audit = cimmeria_upk::patcher::name_audit::audit_client_names(
            &package,
            0..package.exports.len(),
        )
        .unwrap();
        assert_eq!(audit.unloadable, vec![], "{start}");
        assert!(
            audit.not_audited.is_empty(),
            "{start}: {:?}",
            audit.not_audited
        );
        assert!(audit.audited > 700, "{start}: audited {}", audit.audited);

        let built = rig_positions(&tree.path().join(IHPET));
        assert_eq!(built.len(), 8, "{start}: {built:?}");
        for (path, x, z) in seeded_rigs() {
            let (_, bx, bz) = built
                .iter()
                .find(|b| b.0 == path)
                .unwrap_or_else(|| panic!("{path} is not in the rebuilt map: {built:?}"));
            assert!(
                (bx - x).abs() < 0.05 && (bz - z).abs() < 0.05,
                "{start}: {path}: rig at ({bx}, {bz}), seeded pad at ({x}, {z})"
            );
        }
    }
}

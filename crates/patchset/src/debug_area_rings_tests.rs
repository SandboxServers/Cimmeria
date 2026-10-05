//! `010-debug-area-rings`: eight ring transport rigs cloned into one
//! Ihpet_Crater_Light chunk for the GM-only Debug Area (world 1300, DA-08).
//!
//! The rig is region 3's de-prefabbed rig in `Castle_CellBlock-fffeffff`.
//! `007-castle-armory-ring` rewrites that file, so on every launcher install
//! the file on disk is 007's result, not the stock map. 010 therefore pins
//! its donor source to 007's result hash and must be published `after` 007.
//! These tests read the committed zips only; the last one, ignored, applies
//! 010 to a real client.

use std::io::Read;
use std::path::Path;

use crate::recipe::{Recipe, Transform, RECIPE_NAME};

const IHPET: &str =
    "Working/SGWGame/CookedPC/Maps/Ihpet_Crater_Light/Ihpet_Crater_Light-fff80002.umap";
const ARMORY: &str =
    "Working/SGWGame/CookedPC/Maps/Castle_CellBlock/Castle_CellBlock-fffeffff.umap";

fn committed_zip(id: &str) -> zip::ZipArchive<std::fs::File> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/client-patches")
        .join(format!("{id}.zip"));
    zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap()
}

fn entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

fn recipe(id: &str) -> (Recipe, zip::ZipArchive<std::fs::File>) {
    let mut z = committed_zip(id);
    let r = Recipe::parse(&entry(&mut z, RECIPE_NAME)).unwrap();
    (r, z)
}

/// bsdiff 4 (`BSDIFF40`) offsets: sign-magnitude, little-endian.
fn offt(bytes: &[u8]) -> i64 {
    let v = u64::from_le_bytes(bytes.try_into().unwrap());
    let magnitude = (v & 0x7FFF_FFFF_FFFF_FFFF) as i64;
    if v >> 63 == 1 {
        -magnitude
    } else {
        magnitude
    }
}

/// (compressed control, diff, extra block sizes, result size) from a delta's
/// header. The extra block is the only part of a bsdiff delta that is not
/// derived from the source image: bytes it stores reach the result verbatim.
fn bsdiff_blocks(delta: &[u8]) -> (i64, i64, i64, i64) {
    assert_eq!(&delta[..8], b"BSDIFF40", "not a bsdiff 4 delta");
    let (ctrl, diff, new_size) = (
        offt(&delta[8..16]),
        offt(&delta[16..24]),
        offt(&delta[24..32]),
    );
    (ctrl, diff, delta.len() as i64 - 32 - ctrl - diff, new_size)
}

/// The recipe rebuilds exactly one map, the chunk the eight rigs live in,
/// from its own normalized stock bytes plus the Armory map *as 007 leaves
/// it*. Guards a rebuild against the stock Armory map: that source hash would
/// not match any launcher install, since 007 rewrote the file before 010
/// runs, and 010 would fail on everyone.
#[test]
fn committed_010_rebuilds_one_ihpet_chunk_from_its_stock_bytes_and_the_007_armory_map() {
    let (r010, z010) = recipe("010-debug-area-rings");
    let (r007, _) = recipe("007-castle-armory-ring");
    assert_eq!(r010.ops.len(), 1, "{r010:?}");
    let op = &r010.ops[0];
    assert_eq!(op.target, IHPET);
    assert_eq!(op.sources.len(), 2, "{op:?}");
    assert_eq!(op.sources[0].path, IHPET);
    assert_eq!(op.sources[0].transform, Transform::UpkNormalize);
    assert_eq!(op.sources[1].path, ARMORY);
    assert_eq!(op.sources[1].transform, Transform::None);
    let armory_007 = r007.ops.iter().find(|o| o.target == ARMORY).unwrap();
    assert_eq!(
        op.sources[1].sha256, armory_007.result_sha256,
        "010's donor must be 007's result, the Armory map every launcher install holds"
    );
    // Only the recipe and the one delta: no whole files ride along.
    assert_eq!(z010.len(), 2);
}

/// The no-CME-bytes rule, as far as a committed zip can show it: the delta's
/// extra block (bytes copied verbatim into the map) stays at a few hundred
/// compressed bytes of short fragments. A delta built against the raw LZO
/// stock map instead of the normalized one, or one that carried a cloned
/// object graph whole, would put tens of kilobytes there.
#[test]
fn committed_010_delta_ships_no_verbatim_map_bytes() {
    let (r010, mut z010) = recipe("010-debug-area-rings");
    let delta = entry(&mut z010, &r010.ops[0].delta);
    let (_, _, extra, new_size) = bsdiff_blocks(&delta);
    assert!(extra < 2048, "extra block is {extra} compressed bytes");
    assert!(delta.len() < 32 * 1024, "delta is {} bytes", delta.len());
    assert!(new_size > 2_000_000, "result is {new_size} bytes");
}

// Manual check against a real client: SGW_PATCHED_CLIENT = a client that has
// 007 applied (the QA client does) and a stock Ihpet_Crater_Light-fff80002.
// The two source files are copied into a temp tree, 010 is applied there, and
// the rebuilt map must match the recipe's result hash. Run with
//   cargo test -p cimmeria-patchset real_client_debug_area_rings -- --ignored --nocapture
#[test]
#[ignore = "needs a client with 007 applied; see the comment"]
fn real_client_debug_area_rings() {
    let client = std::path::PathBuf::from(std::env::var("SGW_PATCHED_CLIENT").unwrap());
    let tree = tempfile::tempdir().unwrap();
    for rel in [IHPET, ARMORY] {
        let to = tree.path().join(rel);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(client.join(rel), &to).unwrap();
    }
    let zip = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/client-patches/010-debug-area-rings.zip");
    let report = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(report.rebuilt, vec![IHPET.to_string()]);
    let (r010, _) = recipe("010-debug-area-rings");
    let rebuilt = std::fs::read(tree.path().join(IHPET)).unwrap();
    assert_eq!(crate::sha256_hex(&rebuilt), r010.ops[0].result_sha256);
    // A second apply is a no-op.
    let again = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(again.already_current, vec![IHPET.to_string()]);
}

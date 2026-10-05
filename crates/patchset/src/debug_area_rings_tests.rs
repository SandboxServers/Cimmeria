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
    // ...and says so, so a mismatch names 007 instead of "the stock client".
    assert_eq!(op.sources[0].output_of, None);
    assert_eq!(
        op.sources[1].output_of.as_deref(),
        Some("007-castle-armory-ring")
    );
    // Only the recipe and the one delta: no whole files ride along.
    assert_eq!(z010.len(), 2);
}

/// The no-CME-bytes rule, as far as a committed zip can show it. The delta's
/// extra block is the only part not derived from a source; its bytes reach
/// the map verbatim. Today it is 487 bytes compressed (4,163 raw): our
/// coordinates, export and name indices, and about 463 bytes of short CME
/// identifier fragments (`ring5ring4ring3ring2ring1` eight times, `Bool`,
/// `ource`). 007's extra block is 31 bytes. The bound sits just above today's
/// size so growth is noticed: a delta against the raw LZO stock map, or one
/// carrying a cloned object graph whole, puts kilobytes there.
#[test]
fn committed_010_delta_ships_no_verbatim_map_bytes() {
    let (r010, mut z010) = recipe("010-debug-area-rings");
    let delta = entry(&mut z010, &r010.ops[0].delta);
    let (_, _, extra, new_size) = bsdiff_blocks(&delta);
    assert!(extra <= 520, "extra block is {extra} compressed bytes");
    assert!(delta.len() < 32 * 1024, "delta is {} bytes", delta.len());
    assert!(new_size > 2_000_000, "result is {new_size} bytes");
}

/// A source pinned to another patch's output fails with an error that names
/// that patch, not "does not match the stock client ... Reinstall the seed",
/// and leaves the target untouched. A player whose 007 failed or was skipped
/// sees why 010 cannot apply.
#[test]
fn a_donor_that_is_not_007_output_names_patch_007() {
    use crate::build::{SpecOp, SpecSource};
    use crate::tests::{noisy, write};
    use crate::{apply, build, PatchsetError, Spec};

    let dir = tempfile::tempdir().unwrap();
    let (stock, patched, install) = (
        dir.path().join("stock"),
        dir.path().join("patched"),
        dir.path().join("install"),
    );
    let (ihpet, armory_007) = (noisy(21, 30_000), noisy(41, 30_000));
    let mut rebuilt = ihpet.clone();
    rebuilt.extend_from_slice(&armory_007[1_000..5_000]);
    write(&stock, IHPET, &ihpet);
    write(&stock, ARMORY, &armory_007);
    write(&patched, IHPET, &rebuilt);
    let spec = Spec {
        id: "010".into(),
        title: None,
        description: None,
        ops: vec![SpecOp {
            target: IHPET.into(),
            sources: vec![
                SpecSource {
                    path: IHPET.into(),
                    transform: Transform::None,
                    output_of: None,
                },
                SpecSource {
                    path: ARMORY.into(),
                    transform: Transform::None,
                    output_of: Some("007-castle-armory-ring".into()),
                },
            ],
        }],
        files: vec![],
    };
    let zip = dir.path().join("010.zip");
    let built = build::build(&spec, dir.path(), &stock, &patched).unwrap();
    std::fs::write(&zip, built.zip).unwrap();

    // 007 never applied: the Armory map on disk is some other file.
    write(&install, IHPET, &ihpet);
    // (noisy seeds n and n | 1 give the same bytes, so use an unrelated one.)
    write(&install, ARMORY, &noisy(61, 30_000));
    let err = apply(&zip, &install, &mut |_| {}).unwrap_err();
    assert!(
        matches!(&err, PatchsetError::PatchOutputMismatch { path, patch, .. }
            if path == ARMORY && patch == "007-castle-armory-ring"),
        "{err:?}"
    );
    let text = err.to_string();
    assert!(
        text.contains("patch 007-castle-armory-ring's output"),
        "{text}"
    );
    assert!(!text.contains("stock client"), "{text}");
    assert_eq!(std::fs::read(install.join(IHPET)).unwrap(), ihpet);
}

/// `(Teleport Out sequence path, pad x, pad z)` per Debug Area station, read
/// from the seed: region row -> event set -> sequence row.
fn seeded_rigs() -> Vec<(String, f32, f32)> {
    let seed = |rel: &str| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../db/resources");
        std::fs::read_to_string(root.join(rel)).unwrap()
    };
    let values = |sql: &str, table: &str| -> Vec<Vec<String>> {
        let head = format!("INSERT INTO {table} (");
        sql.lines()
            .filter(|l| l.starts_with(&head))
            .map(|l| {
                let v = &l[l.find("VALUES (").unwrap() + 8..l.rfind(");").unwrap()];
                v.split(", ")
                    .map(|f| f.trim_matches('\'').to_string())
                    .collect()
            })
            .collect()
    };
    let events = seed("Events/Seed/debug_area_ring_events.sql");
    let seqs = values(&events, "sequences");
    let links = values(&events, "event_sets_sequences");
    let regions = values(
        &seed("Worlds/Seed/debug_area_rings.sql"),
        "ring_transport_regions",
    );
    regions
        .iter()
        .map(|r| {
            let event_set = &r[8];
            let path = links
                .iter()
                .filter(|l| &l[0] == event_set)
                .find_map(|l| seqs.iter().find(|s| s[0] == l[1] && s[1] == "8000"))
                .map(|s| s[2].clone())
                .unwrap_or_else(|| panic!("event set {event_set} has no Teleport Out"));
            (path, r[2].parse().unwrap(), r[4].parse().unwrap())
        })
        .collect()
}

/// The seed reader itself: eight stations, eight distinct rig paths.
#[test]
fn the_seed_names_eight_distinct_rigs() {
    let rigs = seeded_rigs();
    assert_eq!(rigs.len(), 8, "{rigs:?}");
    let mut paths: Vec<_> = rigs.iter().map(|r| r.0.clone()).collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), 8);
}

/// In a rebuilt map, where the rig behind each `..._Seq[_N]` stands, as game
/// (x, z): the location of an `InterpActor` its `SeqVar_Object`s drive.
fn rig_positions(map: &Path) -> Vec<(String, f32, f32)> {
    use cimmeria_upk::{parse_tagged_properties, Package, PropValue};
    let pkg = Package::open(map).unwrap();
    let prop = |index: usize, offset: usize, name: &str| {
        let data = pkg.read_export_data(&pkg.exports[index]).unwrap();
        parse_tagged_properties(&data, offset, &pkg.names)
            .into_iter()
            .find(|p| p.name == name)
            .map(|p| p.value)
    };
    let mut out = Vec::new();
    for (i, e) in pkg.exports.iter().enumerate() {
        if pkg.export_class_name(e) != "Sequence"
            || e.object_name != "GLB-RingTransporterBase_TC00_Pf0_Seq"
        {
            continue;
        }
        let name = match e.object_name_num {
            0 => e.object_name.clone(),
            n => format!("{}_{}", e.object_name, n - 1),
        };
        let seq_ref = i as i32 + 1;
        let (x, z) = pkg
            .exports
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                v.package_index == seq_ref && pkg.export_class_name(v) == "SeqVar_Object"
            })
            .filter_map(|(vi, _)| match prop(vi, 4, "ObjValue") {
                Some(PropValue::Object(r)) if r > 0 => Some(r as usize - 1),
                _ => None,
            })
            .filter(|&a| pkg.export_class_name(&pkg.exports[a]) == "InterpActor")
            .find_map(|a| match prop(a, 32, "Location") {
                // UE (X, Y, Z) = game (z, x, y) * 100.
                Some(PropValue::Vector { x, y, .. }) => Some((y / 100.0, x / 100.0)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name} drives no InterpActor"));
        out.push((
            format!("Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.{name}"),
            x,
            z,
        ));
    }
    out
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

    // Each seeded sequence path drives the rig on its own pad. The `_Seq` /
    // `_Seq_N` names follow the `--first-at` order the map was built with; a
    // reordered build would play the wrong station's rings.
    let built = rig_positions(&tree.path().join(IHPET));
    assert_eq!(built.len(), 8, "{built:?}");
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

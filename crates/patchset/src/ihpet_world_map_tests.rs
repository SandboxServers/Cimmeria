//! `013-ihpet-world-map`: rebuilds the `world__default_` overview texture of
//! the stock `Ihpet_Crater_Light_MapData.upk` on the player's machine, with
//! the `world_map_rebake` transform (see `data/client-patches/README.md`).
//! The tests read the committed zip; the last one, ignored, applies it to a
//! real client file. The transform itself is pinned in `cimmeria-upk`'s
//! texture tests.

use std::collections::HashSet;
use std::path::Path;

use crate::debug_area_rings_tests::{bsdiff_blocks, entry, recipe};
use crate::recipe::{Recipe, Transform, WorldMapParams};

const ID: &str = "013-ihpet-world-map";
const MAPDATA: &str =
    "Working/SGWGame/CookedPC/Maps/Ihpet_Crater_Light/Ihpet_Crater_Light_MapData.upk";
/// The 2009 file's SHA-256: the one the recipe pins as the source.
const STOCK_SHA256: &str = "ea86f7c32b6d8e230c86191b5f048e080fc979e80d584bbdc878e3db5404a1f4";

fn expected_params() -> WorldMapParams {
    WorldMapParams {
        texture: "world__default_".into(),
        tile_prefix: "thumb_WorldMap_".into(),
        lo: [-3, 7],
        hi: [-13, 0],
        size: 1024,
        pad: [255, 0, 255],
        carry: 8,
    }
}

/// One op, one file, one start: the stock MapData file as it ships, rebuilt by
/// the transform, with no other patch's output. 001-012 never write this file,
/// so the patch has no ordering constraint.
#[test]
fn committed_013_rebuilds_the_stock_map_data_file_with_the_rebake_transform() {
    let (r, z) = recipe(ID);
    assert_eq!(r.ops.len(), 1, "{r:?}");
    let op = &r.ops[0];
    assert_eq!(op.target, MAPDATA);
    assert_eq!(op.sources.len(), 1, "{op:?}");
    assert_eq!(op.sources[0].path, MAPDATA);
    assert_eq!(
        op.sources[0].transform,
        Transform::WorldMapRebake(expected_params())
    );
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

/// The zero-CME-bytes rule: the zip carries a recipe and a delta that only
/// covers what the transform leaves out, so no picture data. A zip that
/// shipped the rebuilt texture (about 420 KB) or the stock package would break
/// these bounds.
#[test]
fn committed_013_zip_carries_no_picture_data() {
    let (r, mut z) = recipe(ID);
    let delta = entry(&mut z, &r.ops[0].delta);
    let (_, _, extra, new_size) = bsdiff_blocks(&delta);
    assert!(
        new_size > 6_000_000,
        "result is {new_size} bytes, the stock file is 6,167,790"
    );
    assert!(extra < 1_024, "extra block is {extra} compressed bytes");
    assert!(delta.len() < 4_096, "delta is {} bytes", delta.len());
    let zip_len = std::fs::metadata(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/client-patches")
            .join(format!("{ID}.zip")),
    )
    .unwrap()
    .len();
    assert!(zip_len < 8_192, "zip is {zip_len} bytes");
}

/// Nothing in CI can run the transform on the real file (it is CME's map
/// data, which the repo does not hold), so the committed bytes are pinned to
/// what the README tells the coordinator to publish: the zip's SHA-256 and
/// size, the result hash and the stock hash.
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
    // The README writes sizes with thousands separators.
    let size = zip.len();
    let size = format!("{},{:03}", size / 1000, size % 1000);
    for (what, value) in [
        ("013 zip sha256", zip_sha),
        ("013 zip size", size),
        ("013 result sha256", r.ops[0].result_sha256.clone()),
        ("stock MapData sha256", STOCK_SHA256.to_string()),
    ] {
        assert!(
            readme.contains(&value),
            "the README does not state the {what} ({value}); \
             update it with the zip, or the zip is not the one it describes"
        );
    }
}

/// The recipe format stays readable by the launchers that know it, and the
/// new transform serializes as a one-key object next to the unit variants'
/// plain strings.
#[test]
fn the_new_transform_round_trips_beside_the_old_ones() {
    for (t, json) in [
        (Transform::None, r#""none""#),
        (Transform::UpkNormalize, r#""upk_normalize""#),
    ] {
        assert_eq!(serde_json::to_string(&t).unwrap(), json);
        assert_eq!(serde_json::from_str::<Transform>(json).unwrap(), t);
    }
    let t = Transform::WorldMapRebake(expected_params());
    let json = serde_json::to_string(&t).unwrap();
    assert!(
        json.starts_with(r#"{"world_map_rebake":{"texture":"world__default_""#),
        "{json}"
    );
    assert_eq!(serde_json::from_str::<Transform>(&json).unwrap(), t);
}

/// The `Transform` of the launchers that predate 013, as published: two unit
/// variants. Reading the committed recipe with it is what such a launcher does.
#[test]
fn a_launcher_with_the_old_transform_enum_rejects_the_committed_recipe_by_name() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(dead_code)]
    enum OldTransform {
        None,
        UpkNormalize,
    }
    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct OldSource {
        transform: OldTransform,
    }
    let (_, mut z) = recipe(ID);
    let json: serde_json::Value =
        serde_json::from_slice(&entry(&mut z, crate::recipe::RECIPE_NAME)).unwrap();
    let source = json["ops"][0]["sources"][0].clone();
    let err = serde_json::from_value::<OldSource>(source)
        .unwrap_err()
        .to_string();
    assert!(err.contains("unknown variant `world_map_rebake`"), "{err}");
}

fn recipe_json(transform: &str) -> String {
    format!(
        r#"{{"schema":1,"ops":[{{"target":"a.bin","sources":[{{"path":"a.bin","sha256":"00","transform":{transform}}}],"delta":"deltas/000.bsdiff","result_sha256":"00"}}]}}"#
    )
}

/// What a launcher that predates the transform does: it cannot parse the
/// recipe, reports an error naming the variant, and applies nothing. The
/// launcher turns that into one failed patch and carries on with the others
/// (`install_tests::a_patch_with_a_transform_this_launcher_does_not_know_fails_alone`).
#[test]
fn an_unknown_transform_fails_the_recipe_cleanly() {
    let err = Recipe::parse(recipe_json(r#"{"future_rebake":{}}"#).as_bytes()).unwrap_err();
    assert!(err.to_string().contains("future_rebake"), "{err}");
    // The same recipe with a transform this build knows parses.
    assert!(Recipe::parse(recipe_json(r#""upk_normalize""#).as_bytes()).is_ok());

    // And a zip carrying it leaves the install untouched.
    let dir = tempfile::tempdir().unwrap();
    let install = dir.path().join("install");
    crate::tests::write(&install, "a.bin", b"stock");
    let zip = dir.path().join("p.zip");
    {
        use std::io::Write;
        let mut zw = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
        zw.start_file(
            crate::recipe::RECIPE_NAME,
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zw.write_all(recipe_json(r#"{"future_rebake":{}}"#).as_bytes())
            .unwrap();
        zw.finish().unwrap();
    }
    let before = std::fs::read(install.join("a.bin")).unwrap();
    assert!(crate::apply(&zip, &install, &mut |_| {}).is_err());
    assert_eq!(std::fs::read(install.join("a.bin")).unwrap(), before);
    assert_eq!(
        std::fs::read_dir(&install).unwrap().count(),
        1,
        "nothing was written"
    );
}

// Manual check against a real client file: SGW_STOCK_MAPDATA = a stock
// Ihpet_Crater_Light_MapData.upk. 013 is applied to a copy and must end on the
// recipe's result hash, a second apply must find it current, and no 64-byte
// run of the rebuilt texture's DXT1 data may appear in the zip.
//   cargo test -p cimmeria-patchset real_client_ihpet_world_map -- --ignored --nocapture
#[test]
#[ignore = "needs the stock Ihpet_Crater_Light_MapData.upk; see the comment"]
fn real_client_ihpet_world_map() {
    let stock = std::path::PathBuf::from(std::env::var("SGW_STOCK_MAPDATA").unwrap());
    let zip_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/client-patches/013-ihpet-world-map.zip");
    let (r, _) = recipe(ID);
    let stock_len = std::fs::metadata(&stock).unwrap().len() as usize;
    let tree = tempfile::tempdir().unwrap();
    let to = tree.path().join(MAPDATA);
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(&stock, &to).unwrap();
    let report = crate::apply(&zip_path, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(report.rebuilt, vec![MAPDATA.to_string()]);
    let rebuilt = std::fs::read(&to).unwrap();
    assert_eq!(crate::sha256_hex(&rebuilt), r.ops[0].result_sha256);
    let again = crate::apply(&zip_path, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(again.already_current, vec![MAPDATA.to_string()]);

    // Everything the transform appended (the new texture and the tables) is
    // absent from the zip, at any alignment.
    let zip = std::fs::read(&zip_path).unwrap();
    let in_zip: HashSet<&[u8]> = zip.windows(64).collect();
    let appended = &rebuilt[stock_len..];
    assert!(appended.len() > 400_000);
    let hits = appended
        .as_chunks::<64>()
        .0
        .iter()
        .filter(|c| in_zip.contains(&c[..]))
        .count();
    assert_eq!(
        hits, 0,
        "{hits} 64-byte runs of the rebuilt file are in the zip"
    );
}

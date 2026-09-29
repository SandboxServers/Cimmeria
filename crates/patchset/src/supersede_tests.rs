//! A patch set that supersedes an earlier one: `007-castle-armory-ring`
//! carries only the Armory op of `002-castle-ring-transport`, which the
//! maintainer retired from the manifest on 2026-09-29 to drop its
//! stasis-hall ring station. Installs that applied 002 and fresh installs
//! must both end with the patched Armory map, and 007 must never touch the
//! stasis-hall map, whichever form it is in.
//!
//! The synthetic tests model the maps with noise; the last test pins the
//! committed zips themselves.

use std::path::{Path, PathBuf};

use crate::build::{SpecOp, SpecSource};
use crate::recipe::{Recipe, Transform, RECIPE_NAME};
use crate::tests::{noisy, write};
use crate::{apply, build, Spec};

const STASIS: &str =
    "Working/SGWGame/CookedPC/Maps/Castle_CellBlock/Castle_CellBlock-fffdfffc.umap";
const ARMORY: &str =
    "Working/SGWGame/CookedPC/Maps/Castle_CellBlock/Castle_CellBlock-fffeffff.umap";

fn op(target: &str, sources: &[&str]) -> SpecOp {
    SpecOp {
        target: target.into(),
        sources: sources
            .iter()
            .map(|p| SpecSource {
                path: (*p).into(),
                transform: Transform::None,
            })
            .collect(),
    }
}

fn spec(id: &str, ops: Vec<SpecOp>) -> Spec {
    Spec {
        id: id.into(),
        title: None,
        description: None,
        ops,
        files: vec![],
    }
}

struct Maps {
    _dir: tempfile::TempDir,
    root: PathBuf,
    stasis_stock: Vec<u8>,
    armory_stock: Vec<u8>,
    stasis_002: Vec<u8>,
    armory_002: Vec<u8>,
    zip_002: PathBuf,
    zip_007: PathBuf,
}

/// Build a 002-shaped zip (stasis from stasis + armory, armory from
/// itself) and a 007-shaped zip (the armory op alone) from the same stock
/// and patched trees, as the real ones were.
fn maps() -> Maps {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let stock = root.join("stock");
    let patched = root.join("patched");

    let stasis_stock = noisy(11, 40_000);
    let armory_stock = noisy(12, 60_000);
    let mut stasis_002 = stasis_stock.clone();
    stasis_002.extend_from_slice(&armory_stock[5_000..9_000]);
    stasis_002.extend_from_slice(b"ring station");
    let mut armory_002 = armory_stock.clone();
    armory_002.extend_from_slice(b"ring rig for mission 688");

    write(&stock, STASIS, &stasis_stock);
    write(&stock, ARMORY, &armory_stock);
    write(&patched, STASIS, &stasis_002);
    write(&patched, ARMORY, &armory_002);

    let zip = |s: &Spec, name: &str| {
        let report = build::build(s, &root, &stock, &patched).unwrap();
        let path = root.join(name);
        std::fs::write(&path, report.zip).unwrap();
        path
    };
    let zip_002 = zip(
        &spec(
            "002",
            vec![op(STASIS, &[STASIS, ARMORY]), op(ARMORY, &[ARMORY])],
        ),
        "002.zip",
    );
    let zip_007 = zip(&spec("007", vec![op(ARMORY, &[ARMORY])]), "007.zip");
    Maps {
        _dir: dir,
        root,
        stasis_stock,
        armory_stock,
        stasis_002,
        armory_002,
        zip_002,
        zip_007,
    }
}

impl Maps {
    /// A fresh stock install under `name`.
    fn install(&self, name: &str) -> PathBuf {
        let dir = self.root.join(name);
        write(&dir, STASIS, &self.stasis_stock);
        write(&dir, ARMORY, &self.armory_stock);
        dir
    }
}

fn read(install: &Path, rel: &str) -> Vec<u8> {
    std::fs::read(install.join(rel)).unwrap()
}

#[test]
fn superseding_patch_on_a_stock_install_patches_only_the_armory() {
    let m = maps();
    let install = m.install("fresh");
    let report = apply(&m.zip_007, &install, &mut |_| {}).unwrap();
    assert_eq!(report.rebuilt, [ARMORY]);
    assert!(report.already_current.is_empty());
    assert_eq!(read(&install, ARMORY), m.armory_002);
    assert_eq!(read(&install, STASIS), m.stasis_stock);
}

/// On an install that applied 002 the armory already has 007's result,
/// so the op is skipped before its (now non-stock) source is checked.
/// Were the source checked first, 007 would fail with SourceMismatch on
/// every 002 install.
#[test]
fn superseding_patch_on_an_install_that_applied_the_old_one_is_skipped() {
    let m = maps();
    let install = m.install("had-002");
    apply(&m.zip_002, &install, &mut |_| {}).unwrap();
    assert_eq!(read(&install, STASIS), m.stasis_002);
    assert_eq!(read(&install, ARMORY), m.armory_002);

    let report = apply(&m.zip_007, &install, &mut |_| {}).unwrap();
    assert!(report.rebuilt.is_empty(), "{report:?}");
    assert_eq!(report.already_current, [ARMORY]);
    assert_eq!(read(&install, ARMORY), m.armory_002);
    // 007 carries no stasis op: a 002 install keeps 002's stasis map.
    assert_eq!(read(&install, STASIS), m.stasis_002);
}

#[test]
fn superseding_patch_applied_twice_changes_nothing() {
    let m = maps();
    let install = m.install("twice");
    apply(&m.zip_007, &install, &mut |_| {}).unwrap();
    let before = (read(&install, STASIS), read(&install, ARMORY));
    let report = apply(&m.zip_007, &install, &mut |_| {}).unwrap();
    assert!(report.rebuilt.is_empty(), "{report:?}");
    assert_eq!(report.already_current, [ARMORY]);
    assert_eq!((read(&install, STASIS), read(&install, ARMORY)), before);
}

fn committed_zip(id: &str) -> zip::ZipArchive<std::fs::File> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/client-patches")
        .join(format!("{id}.zip"));
    zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap()
}

fn entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Vec<u8> {
    use std::io::Read;
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

/// The committed 007 zip rebuilds the Armory map exactly as 002 did (same
/// pinned source, transform, delta and result), so a 002 install skips it
/// and a fresh install gets 002's Armory map; and it never names the
/// stasis-hall map. Guards a rebuild of 007 from the wrong trees.
#[test]
fn committed_007_repeats_the_armory_op_of_002_and_nothing_else() {
    let mut z002 = committed_zip("002-castle-ring-transport");
    let mut z007 = committed_zip("007-castle-armory-ring");
    let r002 = Recipe::parse(&entry(&mut z002, RECIPE_NAME)).unwrap();
    let r007 = Recipe::parse(&entry(&mut z007, RECIPE_NAME)).unwrap();

    let armory_002 = r002.ops.iter().find(|o| o.target == ARMORY).unwrap();
    assert_eq!(r007.ops.len(), 1, "{r007:?}");
    let armory_007 = &r007.ops[0];
    assert_eq!(armory_007.target, armory_002.target);
    assert_eq!(armory_007.sources, armory_002.sources);
    assert_eq!(armory_007.result_sha256, armory_002.result_sha256);
    assert_eq!(
        entry(&mut z007, &armory_007.delta),
        entry(&mut z002, &armory_002.delta)
    );
    assert!(r007
        .ops
        .iter()
        .all(|o| o.target != STASIS && o.sources.iter().all(|s| s.path != STASIS)));
    // Only the recipe and the one delta: no whole files ride along.
    assert_eq!(z007.len(), 2);
}

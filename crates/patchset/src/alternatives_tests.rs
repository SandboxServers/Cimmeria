//! An op with `alternatives`: one result reachable from two starting
//! points. This is how a patch repairs a published patch that broke a
//! file: clean installs rebuild the fixed file from the stock one, installs
//! that applied the broken patch rebuild it from the broken output.
//!
//! The maps are modelled with noise: `stock` is the player's own file,
//! `broken` is what the retired patch wrote, `fixed` is what the repair
//! writes. Both deltas must land on the same bytes.

use std::path::{Path, PathBuf};

use crate::build::{SpecAlternative, SpecOp, SpecSource};
use crate::recipe::{Recipe, Transform, RECIPE_NAME};
use crate::tests::{noisy, write};
use crate::{apply, build, PatchsetError, Spec};

const MAP: &str = "Working/SGWGame/CookedPC/Maps/Area/Area-fff80002.umap";
const DONOR: &str = "Working/SGWGame/CookedPC/Maps/Other/Other-fffeffff.umap";

struct World {
    _dir: tempfile::TempDir,
    root: PathBuf,
    stock: Vec<u8>,
    broken: Vec<u8>,
    fixed: Vec<u8>,
    zip: PathBuf,
}

fn source(path: &str) -> SpecSource {
    SpecSource {
        path: path.into(),
        transform: Transform::None,
        output_of: None,
    }
}

fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let (stock_tree, broken_tree, fixed_tree) =
        (root.join("stock"), root.join("broken"), root.join("fixed"));

    let stock = noisy(21, 60_000);
    let donor = noisy(22, 30_000);
    // The retired patch appended donor bytes plus its own tables.
    let mut broken = stock.clone();
    broken.extend_from_slice(&donor[2_000..9_000]);
    broken.extend_from_slice(b"broken tables");
    // The repair appends a different slice of the donor.
    let mut fixed = stock.clone();
    fixed.extend_from_slice(&donor[2_000..8_000]);
    fixed.extend_from_slice(b"fixed tables");

    write(&stock_tree, MAP, &stock);
    write(&stock_tree, DONOR, &donor);
    write(&broken_tree, MAP, &broken);
    write(&fixed_tree, MAP, &fixed);

    let spec = Spec {
        id: "t-fix".into(),
        title: None,
        description: None,
        ops: vec![SpecOp {
            target: MAP.into(),
            sources: vec![source(MAP), source(DONOR)],
            alternatives: vec![SpecAlternative {
                sources: vec![source(MAP)],
            }],
        }],
        files: vec![],
    };
    let report =
        build::build_with_alternatives(&spec, &root, &stock_tree, &[broken_tree], &fixed_tree)
            .unwrap();
    let zip = root.join("fix.zip");
    std::fs::write(&zip, &report.zip).unwrap();
    World {
        _dir: dir,
        root,
        stock,
        broken,
        fixed,
        zip,
    }
}

/// An install holding `map` (and the donor, as a real install does).
fn install(w: &World, name: &str, map: &[u8]) -> PathBuf {
    let tree = w.root.join(name);
    write(&tree, MAP, map);
    write(
        &tree,
        DONOR,
        &std::fs::read(w.root.join("stock").join(DONOR)).unwrap(),
    );
    tree
}

fn read(tree: &Path) -> Vec<u8> {
    std::fs::read(tree.join(MAP)).unwrap()
}

#[test]
fn a_clean_install_rebuilds_the_fixed_file_from_the_stock_one() {
    let w = world();
    let tree = install(&w, "clean", &w.stock);
    let report = apply(&w.zip, &tree, &mut |_| {}).unwrap();
    assert_eq!(report.rebuilt, vec![MAP.to_string()]);
    assert_eq!(read(&tree), w.fixed);
}

#[test]
fn an_install_that_applied_the_broken_patch_is_repaired_to_the_same_bytes() {
    let w = world();
    let tree = install(&w, "broken-install", &w.broken);
    apply(&w.zip, &tree, &mut |_| {}).unwrap();
    assert_eq!(read(&tree), w.fixed);
}

#[test]
fn the_repair_does_not_need_the_donor_when_starting_from_the_broken_file() {
    // The broken output already holds everything the fixed file reuses.
    let w = world();
    let tree = install(&w, "no-donor", &w.broken);
    std::fs::remove_file(tree.join(DONOR)).unwrap();
    apply(&w.zip, &tree, &mut |_| {}).unwrap();
    assert_eq!(read(&tree), w.fixed);
}

#[test]
fn applying_twice_changes_nothing_the_second_time() {
    let w = world();
    let tree = install(&w, "twice", &w.broken);
    apply(&w.zip, &tree, &mut |_| {}).unwrap();
    let report = apply(&w.zip, &tree, &mut |_| {}).unwrap();
    assert!(report.rebuilt.is_empty());
    assert_eq!(report.already_current, vec![MAP.to_string()]);
}

#[test]
fn a_file_that_is_neither_start_reports_the_primary_mismatch_and_stays_untouched() {
    let w = world();
    let other = noisy(99, 60_000);
    let tree = install(&w, "other", &other);
    let err = apply(&w.zip, &tree, &mut |_| {}).unwrap_err();
    // The stock file is the one a player can restore; name it, not the broken one.
    match err {
        PatchsetError::SourceMismatch { path, .. } => assert_eq!(path, MAP),
        e => panic!("expected the primary's SourceMismatch, got {e}"),
    }
    assert_eq!(read(&tree), other);
}

#[test]
fn the_repair_delta_from_the_broken_file_carries_no_stock_bytes() {
    // Same shape as 010's rule: the zip holds only what Cimmeria authored.
    let w = world();
    let zip = std::fs::File::open(&w.zip).unwrap();
    let mut archive = zip::ZipArchive::new(zip).unwrap();
    let alt = {
        use std::io::Read;
        let mut bytes = Vec::new();
        archive
            .by_name("deltas/000-alt1.bsdiff")
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        bytes
    };
    assert!(alt.len() < 600, "alternative delta is {} bytes", alt.len());
    assert!(w.stock.len() > 50_000);
}

#[test]
fn a_recipe_without_alternatives_serializes_without_the_field() {
    // Published recipes (001-010) must keep their exact bytes when rebuilt.
    let dir = tempfile::tempdir().unwrap();
    let tree = dir.path().join("t");
    write(&tree, MAP, &noisy(5, 1000));
    let mut patched = noisy(5, 1000);
    patched.extend_from_slice(b"x");
    let p = dir.path().join("p");
    write(&p, MAP, &patched);
    let spec = Spec {
        id: "plain".into(),
        title: None,
        description: None,
        ops: vec![SpecOp {
            target: MAP.into(),
            sources: vec![source(MAP)],
            alternatives: vec![],
        }],
        files: vec![],
    };
    let report = build::build(&spec, dir.path(), &tree, &p).unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(report.zip)).unwrap();
    let mut json = String::new();
    std::io::Read::read_to_string(&mut archive.by_name(RECIPE_NAME).unwrap(), &mut json).unwrap();
    assert!(!json.contains("alternatives"), "{json}");
    // And a recipe without the field still parses (launchers' recipes from before).
    Recipe::parse(json.as_bytes()).unwrap();
}

#[test]
fn building_refuses_a_spec_whose_alternatives_have_no_tree() {
    let dir = tempfile::tempdir().unwrap();
    let spec = Spec {
        id: "x".into(),
        title: None,
        description: None,
        ops: vec![SpecOp {
            target: MAP.into(),
            sources: vec![source(MAP)],
            alternatives: vec![SpecAlternative {
                sources: vec![source(MAP)],
            }],
        }],
        files: vec![],
    };
    let err = build::build(&spec, dir.path(), dir.path(), dir.path()).unwrap_err();
    assert!(err.to_string().contains("alternative"), "{err}");
}

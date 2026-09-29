//! Build → apply round trips on synthetic trees. The `UpkNormalize` path
//! is covered by `real_client_ring_maps` against a real client.

use std::path::Path;

use crate::build::{SpecFile, SpecOp, SpecSource};
use crate::recipe::Transform;
use crate::{apply, build, PatchsetError, Spec};

pub(crate) fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

pub(crate) fn noisy(seed: u32, len: usize) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

struct Fixture {
    _dir: tempfile::TempDir,
    stock: std::path::PathBuf,
    patched: std::path::PathBuf,
    spec_dir: std::path::PathBuf,
    spec: Spec,
    ui_stock: Vec<u8>,
    ui_patched: Vec<u8>,
    map_patched: Vec<u8>,
}

/// A stock tree, a patched tree (one text file edited, one "map" built
/// from itself plus a donor, the donor itself unchanged), and a spec with
/// one overlay file of our own.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let stock = dir.path().join("stock");
    let patched = dir.path().join("patched");
    let spec_dir = dir.path().join("spec");

    let ui_stock = b"function Dialog.show()\r\n  -- stock\r\nend\r\n".repeat(40);
    let mut ui_patched = ui_stock.clone();
    ui_patched.extend_from_slice(b"-- Cimmeria: show the speaker portrait\r\n");
    let map_stock = noisy(1, 50_000);
    let donor = noisy(2, 30_000);
    let mut map_patched = map_stock.clone();
    map_patched.extend_from_slice(&donor[1_000..9_000]);
    map_patched.extend_from_slice(b"new tables");

    for root in [&stock, &patched] {
        write(root, "Working/Binaries/SGW.exe", b"MZ");
        write(root, "Working/SGWGame/Maps/donor.umap", &donor);
    }
    write(&stock, "Working/SGWGame/UI/Dialog.lua", &ui_stock);
    write(&stock, "Working/SGWGame/Maps/cell.umap", &map_stock);
    write(&patched, "Working/SGWGame/UI/Dialog.lua", &ui_patched);
    write(&patched, "Working/SGWGame/Maps/cell.umap", &map_patched);
    write(&spec_dir, "SGWLogConfig.xml", b"<ours/>");

    let spec = Spec {
        id: "t".into(),
        title: None,
        description: None,
        ops: vec![
            SpecOp {
                target: "Working/SGWGame/UI/Dialog.lua".into(),
                sources: vec![SpecSource {
                    path: "Working/SGWGame/UI/Dialog.lua".into(),
                    transform: Transform::None,
                }],
            },
            SpecOp {
                target: "Working/SGWGame/Maps/cell.umap".into(),
                sources: vec![
                    SpecSource {
                        path: "Working/SGWGame/Maps/cell.umap".into(),
                        transform: Transform::None,
                    },
                    SpecSource {
                        path: "Working/SGWGame/Maps/donor.umap".into(),
                        transform: Transform::None,
                    },
                ],
            },
        ],
        files: vec![SpecFile {
            path: "Working/Binaries/SGWLogConfig.xml".into(),
            from: "SGWLogConfig.xml".into(),
        }],
    };
    Fixture {
        _dir: dir,
        stock,
        patched,
        spec_dir,
        spec,
        ui_stock,
        ui_patched,
        map_patched,
    }
}

fn zip_to_disk(f: &Fixture) -> std::path::PathBuf {
    let report = build::build(&f.spec, &f.spec_dir, &f.stock, &f.patched).unwrap();
    let path = f.stock.parent().unwrap().join("p.zip");
    std::fs::write(&path, &report.zip).unwrap();
    path
}

#[test]
fn applying_to_a_stock_install_reproduces_the_patched_files() {
    let f = fixture();
    let zip = zip_to_disk(&f);
    let mut seen = Vec::new();
    let report = apply(&zip, &f.stock, &mut |p| seen.push(p.to_string())).unwrap();
    assert_eq!(report.rebuilt.len(), 2);
    assert_eq!(report.overlay_files, 1);
    assert_eq!(seen.len(), 3);
    let read = |rel: &str| std::fs::read(f.stock.join(rel)).unwrap();
    assert_eq!(read("Working/SGWGame/UI/Dialog.lua"), f.ui_patched);
    assert_eq!(read("Working/SGWGame/Maps/cell.umap"), f.map_patched);
    assert_eq!(read("Working/Binaries/SGWLogConfig.xml"), b"<ours/>");
}

// The redistribution property: the zip must not carry the stock files'
// bytes. The patched map is 50 KB of stock noise plus 8 KB of donor
// noise; both are sources, so only a few hundred bytes should ship.
#[test]
fn the_zip_carries_deltas_not_the_stock_bytes() {
    let f = fixture();
    let report = build::build(&f.spec, &f.spec_dir, &f.stock, &f.patched).unwrap();
    let map_delta = report
        .deltas
        .iter()
        .find(|(t, _, _)| t.ends_with("cell.umap"))
        .unwrap()
        .1;
    assert!(map_delta < 2_000, "map delta {map_delta} bytes");
    assert!(report.zip.len() < 4_000, "zip {} bytes", report.zip.len());
    assert!(
        !report
            .zip
            .windows(64)
            .any(|w| w == &f.map_patched[20_000..20_064]),
        "stock map bytes leaked into the zip"
    );
}

#[test]
fn a_second_apply_changes_nothing() {
    let f = fixture();
    let zip = zip_to_disk(&f);
    apply(&zip, &f.stock, &mut |_| {}).unwrap();
    let report = apply(&zip, &f.stock, &mut |_| {}).unwrap();
    assert!(report.rebuilt.is_empty());
    assert_eq!(report.already_current.len(), 2);
}

// A source that isn't the stock file must fail before anything is
// written, so a half-patched install can't happen.
#[test]
fn a_non_stock_source_fails_and_writes_nothing() {
    let f = fixture();
    let zip = zip_to_disk(&f);
    write(&f.stock, "Working/SGWGame/Maps/donor.umap", b"modded");
    let err = apply(&zip, &f.stock, &mut |_| {}).unwrap_err();
    assert!(
        matches!(&err, PatchsetError::SourceMismatch { path, .. } if path.ends_with("donor.umap")),
        "{err:?}"
    );
    assert_eq!(
        std::fs::read(f.stock.join("Working/SGWGame/UI/Dialog.lua")).unwrap(),
        f.ui_stock,
        "the op before the failing one must not have been written"
    );
    assert!(!f.stock.join("Working/Binaries/SGWLogConfig.xml").exists());
}

// The ring-transport shape: map A is rebuilt from A plus donor B, and B
// is itself patched. Listed B-first, B must still be written last, or a
// crash between the writes leaves A's source already replaced.
#[test]
fn a_target_other_ops_read_is_written_last() {
    let f = fixture();
    write(
        &f.patched,
        "Working/SGWGame/Maps/donor.umap",
        b"patched donor",
    );
    let donor_op = SpecOp {
        target: "Working/SGWGame/Maps/donor.umap".into(),
        sources: vec![SpecSource {
            path: "Working/SGWGame/Maps/donor.umap".into(),
            transform: Transform::None,
        }],
    };
    let mut spec = f.spec.clone();
    spec.ops.insert(0, donor_op);
    let report = build::build(&spec, &f.spec_dir, &f.stock, &f.patched).unwrap();
    let zip = f.stock.parent().unwrap().join("order.zip");
    std::fs::write(&zip, &report.zip).unwrap();
    let mut order = Vec::new();
    apply(&zip, &f.stock, &mut |p| order.push(p.to_string())).unwrap();
    let pos = |needle: &str| order.iter().position(|p| p.ends_with(needle)).unwrap();
    assert!(pos("donor.umap") > pos("cell.umap"), "{order:?}");
    assert_eq!(
        std::fs::read(f.stock.join("Working/SGWGame/Maps/cell.umap")).unwrap(),
        f.map_patched
    );
}

// Bug shape: the bsdiff header's target size went straight into
// `Vec::with_capacity`, so a corrupt delta claiming 1 TiB aborted the
// process instead of returning an error.
#[test]
fn a_delta_claiming_a_huge_target_does_not_abort() {
    let source = noisy(3, 4_096);
    let mut target = source.clone();
    target.extend_from_slice(b"tail");
    let mut delta = Vec::new();
    qbsdiff::Bsdiff::new(&source, &target)
        .compare(std::io::Cursor::new(&mut delta))
        .unwrap();
    // BSDIFF40 header: magic, control length, diff length, target size.
    delta[24..32].copy_from_slice(&(1u64 << 40).to_le_bytes());
    let _ = crate::apply::bspatch(&source, &delta);
}

#[test]
fn a_missing_source_names_the_file() {
    let f = fixture();
    let zip = zip_to_disk(&f);
    std::fs::remove_file(f.stock.join("Working/SGWGame/Maps/cell.umap")).unwrap();
    assert!(matches!(
        apply(&zip, &f.stock, &mut |_| {}),
        Err(PatchsetError::SourceMissing { .. })
    ));
}

#[test]
fn builds_are_byte_for_byte_reproducible() {
    let f = fixture();
    let a = build::build(&f.spec, &f.spec_dir, &f.stock, &f.patched).unwrap();
    let b = build::build(&f.spec, &f.spec_dir, &f.stock, &f.patched).unwrap();
    assert_eq!(a.zip, b.zip);
}

#[test]
fn a_zip_without_a_recipe_is_a_plain_overlay() {
    let f = fixture();
    let spec = Spec {
        id: "o".into(),
        title: None,
        description: None,
        ops: vec![],
        files: f.spec.files.clone(),
    };
    let report = build::build(&spec, &f.spec_dir, &f.stock, &f.patched).unwrap();
    let path = f.stock.parent().unwrap().join("o.zip");
    std::fs::write(&path, &report.zip).unwrap();
    assert!(!crate::apply::has_recipe(&path).unwrap());
    let r = apply(&path, &f.stock, &mut |_| {}).unwrap();
    assert_eq!(r.overlay_files, 1);
}

/// The file names in `dir`, exactly as the directory listing spells them.
fn listed(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

// Bug shape (2026-09-29, launcher-20260929-f518b57): `005-login-delay`'s
// recipe names `eula.lua`, the stock client ships `EULA.lua`, and the
// write-temp-then-rename replaced the stock file under the recipe's
// spelling. The game's UI loader then reported "'EULA.lua' does not exist
// in group lua" and never showed the login screen. The patched file must
// keep the install's own name. `exists()` can't see this on Windows, so
// the assertions read the directory listing.
#[test]
fn a_target_spelled_in_another_case_keeps_the_stock_name() {
    let dir = tempfile::tempdir().unwrap();
    let (stock, patched, install, spec_dir) = (
        dir.path().join("stock"),
        dir.path().join("patched"),
        dir.path().join("install"),
        dir.path().join("spec"),
    );
    let eula_dir = "Working/SGWGame/Content/UI/Startup/EULA";
    let stock_bytes = b"function EULA.onLoad()\r\n  show()\r\nend\r\n".repeat(20);
    let mut patched_bytes = stock_bytes.clone();
    patched_bytes.extend_from_slice(b"-- Cimmeria: wait for the gate intro\r\n");
    // Built the way the published zip was: from trees spelling it `eula.lua`.
    write(&stock, &format!("{eula_dir}/eula.lua"), &stock_bytes);
    write(&patched, &format!("{eula_dir}/eula.lua"), &patched_bytes);
    // Applied to a stock install, which spells it `EULA.lua`.
    write(&install, &format!("{eula_dir}/EULA.lua"), &stock_bytes);
    write(&spec_dir, "SGWLogConfig.xml", b"<ours/>");
    // The stock directory is `binaries`, the overlay entry says `Binaries`,
    // and the player may already have the overlay file under another case.
    write(&install, "Working/binaries/sgwlogconfig.xml", b"<old/>");
    let spec = Spec {
        id: "case".into(),
        title: None,
        description: None,
        ops: vec![SpecOp {
            target: format!("{eula_dir}/eula.lua"),
            sources: vec![SpecSource {
                path: format!("{eula_dir}/eula.lua"),
                transform: Transform::None,
            }],
        }],
        files: vec![SpecFile {
            path: "Working/Binaries/SGWLogConfig.xml".into(),
            from: "SGWLogConfig.xml".into(),
        }],
    };
    let report = build::build(&spec, &spec_dir, &stock, &patched).unwrap();
    let zip = dir.path().join("case.zip");
    std::fs::write(&zip, &report.zip).unwrap();

    let applied = apply(&zip, &install, &mut |_| {}).unwrap();
    assert_eq!(applied.rebuilt.len(), 1);
    assert_eq!(listed(&install.join(eula_dir)), ["EULA.lua"]);
    assert_eq!(
        std::fs::read(install.join(eula_dir).join("EULA.lua")).unwrap(),
        patched_bytes
    );
    assert_eq!(listed(&install.join("Working")), ["SGWGame", "binaries"]);
    assert_eq!(
        listed(&install.join("Working/binaries")),
        ["sgwlogconfig.xml"]
    );
    assert_eq!(
        std::fs::read(install.join("Working/binaries/sgwlogconfig.xml")).unwrap(),
        b"<ours/>"
    );

    // A second run finds the result under the stock name.
    let again = apply(&zip, &install, &mut |_| {}).unwrap();
    assert!(again.rebuilt.is_empty());
    assert_eq!(again.already_current.len(), 1);
}

// The other half of the eula.lua bug: nothing stopped a spec spelled in
// the wrong case from being built, because Windows reads `eula.lua` from a
// stock `EULA.lua` without complaint. `build` now refuses it and names the
// stock spelling.
#[test]
fn a_spec_path_in_another_case_than_the_stock_tree_is_refused() {
    let f = fixture();
    let mut spec = f.spec.clone();
    spec.ops[0].target = "Working/SGWGame/UI/dialog.lua".into();
    spec.ops[0].sources[0].path = "Working/SGWGame/UI/dialog.lua".into();
    let err = build::build(&spec, &f.spec_dir, &f.stock, &f.patched).unwrap_err();
    assert!(
        matches!(&err, PatchsetError::CaseMismatch { path, on_disk }
            if path == "Working/SGWGame/UI/dialog.lua"
                && on_disk == "Working/SGWGame/UI/Dialog.lua"),
        "{err:?}"
    );

    // Directories count too: the stock tree says `Binaries` here.
    let mut spec = f.spec.clone();
    spec.files[0].path = "Working/binaries/SGWLogConfig.xml".into();
    let err = build::build(&spec, &f.spec_dir, &f.stock, &f.patched).unwrap_err();
    assert!(
        matches!(&err, PatchsetError::CaseMismatch { on_disk, .. }
            if on_disk == "Working/Binaries/SGWLogConfig.xml"),
        "{err:?}"
    );
}

// Manual check against a real client: SGW_STOCK_CLIENT = a stock install
// (the launcher's seed, SourceCache renamed), SGW_PATCHED_CLIENT = a client
// with the ring-transport maps. Run with
//   cargo test -p cimmeria-patchset real_client_ring_maps -- --ignored --nocapture
#[test]
#[ignore = "needs a stock and a patched client; see the comment"]
fn real_client_ring_maps() {
    let stock = std::path::PathBuf::from(std::env::var("SGW_STOCK_CLIENT").unwrap());
    let patched = std::path::PathBuf::from(std::env::var("SGW_PATCHED_CLIENT").unwrap());
    let m = "Working/SGWGame/CookedPC/Maps/Castle_CellBlock";
    let norm = |p: String| SpecSource {
        path: p,
        transform: Transform::UpkNormalize,
    };
    let spec = Spec {
        id: "ring".into(),
        title: None,
        description: None,
        ops: vec![SpecOp {
            target: format!("{m}/Castle_CellBlock-fffeffff.umap"),
            sources: vec![norm(format!("{m}/Castle_CellBlock-fffeffff.umap"))],
        }],
        files: vec![],
    };
    let report = build::build(&spec, &stock, &stock, &patched).unwrap();
    println!("{:?}", report.deltas);
    assert!(report.zip.len() < 64 * 1024);
}

//! Build tests: a seed zip and two patch zips written to disk, built into an
//! index, then checked against the manifest that describes them.

use std::io::Write;
use std::path::Path;

use cimmeria_patchset::build::{Spec, SpecOp, SpecSource};
use cimmeria_patchset::Transform;
use tokio_util::sync::CancellationToken;

use super::{build, sha256_hex};
use crate::install_progress::ProgressSink;
use crate::manifest::{Manifest, PatchEntry, SeedEntry};
use crate::unpack::UnpackSink;

const EXE: &str = "Working/Binaries/SGW.exe";
const A: &str = "Working/SGWGame/Data/a.bin";
const B: &str = "Working/SGWGame/Data/b.bin";
const C: &str = "Working/SGWGame/Data/c.bin";

/// The smallest PE that `aslr::disable` accepts: an MZ header pointing at a
/// PE32 optional header with the DYNAMIC_BASE bit set.
fn minimal_exe() -> Vec<u8> {
    let mut exe = vec![0u8; 0x200];
    exe[..2].copy_from_slice(b"MZ");
    exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
    exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
    exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
    exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
    exe[0x186..0x188].copy_from_slice(&0x40u16.to_le_bytes());
    exe
}

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, body) in entries {
        zw.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zw.write_all(body).unwrap();
    }
    zw.finish().unwrap().into_inner()
}

fn write_under(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// A patch set that rebuilds `C` from `A` by delta, built the way the tool
/// builds one: a stock tree holding `A`, a patched tree holding `C`.
fn patch_set(dir: &Path, a: &[u8], c: &[u8]) -> Vec<u8> {
    let stock = dir.join("stock");
    let patched = dir.join("patched");
    write_under(&stock, A, a);
    write_under(&patched, C, c);
    let spec = Spec {
        id: "p1".into(),
        title: None,
        description: None,
        ops: vec![SpecOp {
            alternatives: vec![],
            target: C.into(),
            sources: vec![SpecSource {
                path: A.into(),
                transform: Transform::None,
                output_of: None,
            }],
        }],
        files: vec![],
    };
    cimmeria_patchset::build::build(&spec, dir, &stock, &patched)
        .unwrap()
        .zip
}

/// Test-local sink: progress goes to a watch channel nobody reads.
fn sink() -> UnpackSink {
    UnpackSink {
        progress: ProgressSink::latest().0,
        label: "test".into(),
        cancel: CancellationToken::new(),
    }
}

fn entry(id: &str, blob: &str, bytes: &[u8]) -> PatchEntry {
    PatchEntry {
        id: id.into(),
        blob: blob.into(),
        size: bytes.len() as u64,
        sha256: sha256_hex(bytes),
        after: None,
        root: Default::default(),
        title: None,
        description: None,
    }
}

#[test]
fn builds_the_seed_and_each_patch_layer_then_binds_to_the_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let a = b"alpha stock bytes ".repeat(64);
    let c = [a.as_slice(), b"+ patched tail"].concat();
    let b_stock = b"bravo stock".to_vec();
    let b_overlay = b"bravo replaced by the overlay".to_vec();
    let exe = minimal_exe();

    let seed = zip_of(&[(EXE, &exe), (A, &a), (B, &b_stock)]);
    let seed_path = root.join("seed.zip");
    std::fs::write(&seed_path, &seed).unwrap();

    let p1 = patch_set(root, &a, &c);
    let p1_path = root.join("p1.zip");
    std::fs::write(&p1_path, &p1).unwrap();

    let overlay = zip_of(&[(B, &b_overlay)]);
    let overlay_path = root.join("overlay.zip");
    std::fs::write(&overlay_path, &overlay).unwrap();

    let entries = [
        entry("p1", "p1.zip", &p1),
        entry("overlay", "overlay.zip", &overlay),
    ];
    let manifest = Manifest {
        schema: 1,
        min_launcher: None,
        seed: SeedEntry {
            blob: "seed.zip".into(),
            size: seed.len() as u64,
            sha256: sha256_hex(&seed),
        },
        patches: entries.to_vec(),
        file_index: None,
    };
    let patches = vec![
        (entries[0].clone(), p1_path),
        (entries[1].clone(), overlay_path),
    ];
    let work = root.join("work");

    let index = build(&seed_path, &patches, &manifest, &work, &sink()).unwrap();

    // The seed: the three files of the seed zip, as the scan sees them.
    let mut seed_paths: Vec<&str> = index.seed.files.iter().map(|f| f.path.as_str()).collect();
    seed_paths.sort_unstable();
    assert_eq!(seed_paths.len(), 3, "{seed_paths:?}");
    assert!(seed_paths.contains(&EXE), "{seed_paths:?}");
    assert!(seed_paths.contains(&A), "{seed_paths:?}");
    assert!(seed_paths.contains(&B), "{seed_paths:?}");

    // Patch 1 reads A at its seed bytes and writes only C.
    assert_eq!(index.patches.len(), 2);
    let layer1 = &index.patches[0];
    assert_eq!(layer1.state_key, "p1");
    assert_eq!(layer1.inputs.len(), 1, "{:?}", layer1.inputs);
    assert_eq!(layer1.inputs[0].path, A);
    assert_eq!(layer1.inputs[0].size, a.len() as u64);
    assert_eq!(layer1.inputs[0].sha256, sha256_hex(&a));
    assert_eq!(layer1.outputs.len(), 1, "{:?}", layer1.outputs);
    assert_eq!(layer1.outputs[0].path, C);
    assert_eq!(layer1.outputs[0].sha256, sha256_hex(&c));

    // The overlay has no recipe, so no inputs; it writes B at its new bytes.
    let layer2 = &index.patches[1];
    assert_eq!(layer2.state_key, "overlay");
    assert!(layer2.inputs.is_empty(), "{:?}", layer2.inputs);
    assert_eq!(layer2.outputs.len(), 1, "{:?}", layer2.outputs);
    assert_eq!(layer2.outputs[0].path, B);
    assert_eq!(layer2.outputs[0].size, b_overlay.len() as u64);
    assert_eq!(layer2.outputs[0].sha256, sha256_hex(&b_overlay));

    // Two setup rules: the ASLR copy of SGW.exe and the login servers file.
    assert_eq!(index.setup.len(), 2);

    index.validate().unwrap();
    index.bind(&manifest).unwrap();
    assert!(work.join("tree").join(A).is_file());
}

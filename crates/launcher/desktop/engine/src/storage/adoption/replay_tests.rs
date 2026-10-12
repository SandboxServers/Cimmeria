use super::replay::{estimate, plan, ReplayPlan};
use super::verification::classify;
use super::*;
use crate::manifest::{Manifest, PatchEntry, SeedEntry};
use cimmeria_launcher_core::file_index::{FileIndex, IndexedFile, PatchLayer, SeedLayer};
use sha2::{Digest, Sha256};

const SEED_LEVELS: &[u8] = b"levels as the seed ships them";
const SEED_OVERLAY: &[u8] = b"overlay as the seed ships it";
const OVERLAY_PATCHED: &[u8] = b"overlay after patch 0";
const SET_OUTPUT: &[u8] = b"patch-set output, built from the seed levels";
const EDITED: &[u8] = b"edited by the player";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn file(path: &str, bytes: &[u8]) -> IndexedFile {
    IndexedFile {
        path: path.into(),
        size: bytes.len() as u64,
        sha256: hex(Sha256::digest(bytes).as_slice()),
    }
}

/// Patch 0 is an overlay with no inputs. Patch 1 is a patch set: it reads the
/// seed's `Levels.bin` and writes the new `Patched.bin`.
fn index() -> FileIndex {
    FileIndex {
        schema_version: 1,
        setup_policy_version: 1,
        seed: SeedLayer {
            sha256: hex(Sha256::digest(b"seed archive").as_slice()),
            files: vec![
                file("Working/Data/Levels.bin", SEED_LEVELS),
                file("Working/Data/Overlay.dat", SEED_OVERLAY),
            ],
        },
        patches: vec![
            PatchLayer {
                state_key: "overlay".into(),
                sha256: hex(Sha256::digest(b"patch 0").as_slice()),
                inputs: vec![],
                outputs: vec![file("Working/Data/Overlay.dat", OVERLAY_PATCHED)],
            },
            PatchLayer {
                state_key: "set".into(),
                sha256: hex(Sha256::digest(b"patch 1").as_slice()),
                inputs: vec![file("Working/Data/Levels.bin", SEED_LEVELS)],
                outputs: vec![file("Working/Data/Patched.bin", SET_OUTPUT)],
            },
        ],
        setup: vec![],
    }
}

fn manifest(index: &FileIndex) -> Manifest {
    let patch = |layer: &PatchLayer, size: u64| PatchEntry {
        id: layer.state_key.clone(),
        blob: format!("{}.zip", layer.state_key),
        size,
        sha256: layer.sha256.clone(),
        after: None,
        root: Default::default(),
        title: None,
        description: None,
    };
    Manifest {
        schema: 1,
        min_launcher: None,
        seed: SeedEntry {
            blob: "seed.zip".into(),
            size: 1000,
            sha256: index.seed.sha256.clone(),
        },
        patches: vec![patch(&index.patches[0], 100), patch(&index.patches[1], 200)],
        file_index: None,
    }
}

/// Writes `files` under a fresh folder, scans it, and classifies it against
/// `index` the way verification does.
fn classify_folder(
    index: &FileIndex,
    files: &[(&str, &[u8])],
) -> (tempfile::TempDir, inventory::Index, Vec<Difference>) {
    let dir = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let target = dir.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, bytes).unwrap();
    }
    let scanned = inventory::scan(dir.path(), &CancellationToken::new()).unwrap();
    let found = classify(&scanned, index, &[]);
    (dir, scanned, found)
}

fn replay(files: &[(&str, &[u8])], mode: AdoptionMode) -> ReplayPlan {
    let index = index();
    let (_dir, scanned, found) = classify_folder(&index, files);
    plan(&index, &found, &scanned, mode)
}

#[test]
fn a_stock_folder_at_seed_state_needs_both_patches_and_no_seed() {
    let replayed = replay(
        &[
            ("Working/Data/Levels.bin", SEED_LEVELS),
            ("Working/Data/Overlay.dat", SEED_OVERLAY),
        ],
        AdoptionMode::InPlace,
    );
    assert_eq!(replayed.patches, vec![0, 1]);
    assert!(!replayed.needs_seed);
    assert_eq!(
        replayed.targets,
        vec!["Working/Data/Overlay.dat", "Working/Data/Patched.bin"]
    );
    // The patch set's input is the folder's own Levels.bin, so it is local.
    assert_eq!(
        replayed
            .local
            .get("Working/Data/Levels.bin")
            .map(String::as_str),
        Some("Working/Data/Levels.bin")
    );
}

#[test]
fn a_modified_overlay_output_needs_only_its_patch() {
    let replayed = replay(
        &[
            ("Working/Data/Levels.bin", SEED_LEVELS),
            ("Working/Data/Overlay.dat", EDITED),
            ("Working/Data/Patched.bin", SET_OUTPUT),
        ],
        AdoptionMode::InPlace,
    );
    assert_eq!(replayed.patches, vec![0]);
    assert!(!replayed.needs_seed);
    assert_eq!(replayed.targets, vec!["Working/Data/Overlay.dat"]);
}

#[test]
fn a_modified_patch_set_output_with_seed_input_in_the_folder_needs_no_seed() {
    let replayed = replay(
        &[
            ("Working/Data/Levels.bin", SEED_LEVELS),
            ("Working/Data/Overlay.dat", OVERLAY_PATCHED),
            ("Working/Data/Patched.bin", EDITED),
        ],
        AdoptionMode::InPlace,
    );
    assert_eq!(replayed.patches, vec![1]);
    assert!(!replayed.needs_seed);
    assert_eq!(replayed.targets, vec!["Working/Data/Patched.bin"]);
    assert!(replayed.local.contains_key("Working/Data/Levels.bin"));
}

#[test]
fn a_modified_patch_set_input_needs_the_seed() {
    let replayed = replay(
        &[
            ("Working/Data/Levels.bin", EDITED),
            ("Working/Data/Overlay.dat", OVERLAY_PATCHED),
            ("Working/Data/Patched.bin", EDITED),
        ],
        AdoptionMode::InPlace,
    );
    assert_eq!(replayed.patches, vec![1]);
    assert!(replayed.needs_seed);
    assert!(!replayed.local.contains_key("Working/Data/Levels.bin"));
}

#[test]
fn an_input_at_an_earlier_patch_state_recurses_into_that_patch() {
    // Patch 1 reads Overlay.dat, written by patch 0: its pre-patch-1 bytes are
    // patch 0's output, so patch 0 is needed when the folder does not hold them.
    let mut index = index();
    index.patches[1].inputs = vec![file("Working/Data/Overlay.dat", OVERLAY_PATCHED)];
    let (_dir, scanned, found) = classify_folder(
        &index,
        &[
            ("Working/Data/Overlay.dat", SEED_OVERLAY),
            ("Working/Data/Levels.bin", SEED_LEVELS),
        ],
    );
    let replayed = plan(&index, &found, &scanned, AdoptionMode::InPlace);
    assert_eq!(replayed.patches, vec![0, 1]);
    assert!(!replayed.needs_seed);
}

#[test]
fn a_copy_mode_known_transform_at_another_spelling_is_a_target() {
    let files = [
        ("Working/Data/levels.bin", SEED_LEVELS),
        ("Working/Data/Overlay.dat", OVERLAY_PATCHED),
        ("Working/Data/Patched.bin", SET_OUTPUT),
    ];
    assert!(replay(&files, AdoptionMode::InPlace).targets.is_empty());
    let copied = replay(&files, AdoptionMode::Copy);
    assert_eq!(copied.targets, vec!["Working/Data/Levels.bin"]);
    assert!(copied.patches.is_empty());
    assert!(copied.needs_seed);
}

#[test]
fn the_estimate_sums_needed_sizes_and_drops_a_local_seed() {
    let index = index();
    let manifest = manifest(&index);
    let (_dir, scanned, found) = classify_folder(
        &index,
        &[
            ("Working/Data/Levels.bin", EDITED),
            ("Working/Data/Overlay.dat", EDITED),
        ],
    );
    let replayed = plan(&index, &found, &scanned, AdoptionMode::InPlace);
    assert!(replayed.needs_seed);

    let downloaded = estimate(&replayed, &manifest, false, |_| false);
    assert!(downloaded.seed);
    assert_eq!(downloaded.patches, vec!["overlay", "set"]);
    assert_eq!(downloaded.bytes, 100 + 200 + 1000);

    let local = estimate(&replayed, &manifest, true, |_| false);
    assert!(!local.seed);
    assert_eq!(local.bytes, 100 + 200);

    let stored_overlay = estimate(&replayed, &manifest, false, |sha| {
        sha == manifest.patches[0].sha256
    });
    assert_eq!(stored_overlay.patches, vec!["set"]);
    assert_eq!(stored_overlay.bytes, 200 + 1000);
}

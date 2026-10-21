use super::replay::{estimate, plan, ReplayPlan};
use super::verification::classify;
use super::*;
use crate::install_progress::ProgressSink;
use crate::manifest::{Manifest, PatchEntry, SeedEntry};
use crate::unpack::UnpackSink;
use cimmeria_launcher_core::file_index::{FileIndex, IndexedFile, PatchLayer, SeedLayer};
use cimmeria_launcher_core::seed_source::verify_local_seed;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

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

fn replay(files: &[(&str, &[u8])]) -> ReplayPlan {
    let index = index();
    let (_dir, scanned, found) = classify_folder(&index, files);
    plan(&index, &found, &scanned)
}

#[test]
fn a_stock_folder_at_seed_state_needs_both_patches_and_no_seed() {
    let replayed = replay(&[
        ("Working/Data/Levels.bin", SEED_LEVELS),
        ("Working/Data/Overlay.dat", SEED_OVERLAY),
    ]);
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
    let replayed = replay(&[
        ("Working/Data/Levels.bin", SEED_LEVELS),
        ("Working/Data/Overlay.dat", EDITED),
        ("Working/Data/Patched.bin", SET_OUTPUT),
    ]);
    assert_eq!(replayed.patches, vec![0]);
    assert!(!replayed.needs_seed);
    assert_eq!(replayed.targets, vec!["Working/Data/Overlay.dat"]);
}

#[test]
fn a_modified_patch_set_output_with_seed_input_in_the_folder_needs_no_seed() {
    let replayed = replay(&[
        ("Working/Data/Levels.bin", SEED_LEVELS),
        ("Working/Data/Overlay.dat", OVERLAY_PATCHED),
        ("Working/Data/Patched.bin", EDITED),
    ]);
    assert_eq!(replayed.patches, vec![1]);
    assert!(!replayed.needs_seed);
    assert_eq!(replayed.targets, vec!["Working/Data/Patched.bin"]);
    assert!(replayed.local.contains_key("Working/Data/Levels.bin"));
}

#[test]
fn a_modified_patch_set_input_needs_the_seed() {
    let replayed = replay(&[
        ("Working/Data/Levels.bin", EDITED),
        ("Working/Data/Overlay.dat", OVERLAY_PATCHED),
        ("Working/Data/Patched.bin", EDITED),
    ]);
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
    let replayed = plan(&index, &found, &scanned);
    assert_eq!(replayed.patches, vec![0, 1]);
    assert!(!replayed.needs_seed);
}

#[test]
fn a_known_transform_at_another_spelling_is_never_a_target() {
    // The folder's `levels.bin` already holds the release bytes under another
    // spelling, so nothing is planned and no seed is needed in either mode.
    let replayed = replay(&[
        ("Working/Data/levels.bin", SEED_LEVELS),
        ("Working/Data/Overlay.dat", OVERLAY_PATCHED),
        ("Working/Data/Patched.bin", SET_OUTPUT),
    ]);
    assert!(replayed.targets.is_empty());
    assert!(replayed.patches.is_empty());
    assert!(!replayed.needs_seed);
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
    let replayed = plan(&index, &found, &scanned);
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

#[test]
fn the_estimate_flags_a_local_seed_only_when_it_replaces_one() {
    let index = index();
    let manifest = manifest(&index);
    let (_dir, scanned, found) = classify_folder(
        &index,
        &[
            ("Working/Data/Levels.bin", EDITED),
            ("Working/Data/Overlay.dat", EDITED),
        ],
    );
    let replayed = plan(&index, &found, &scanned);
    assert!(replayed.needs_seed);

    let downloaded = estimate(&replayed, &manifest, false, |_| false);
    assert!(!downloaded.local_seed);

    let local = estimate(&replayed, &manifest, true, |_| false);
    assert!(local.local_seed);
    assert!(!local.seed);
}

// Real tiny artifacts: a two-file seed and one overlay patch per file. The
// blobs are served and verified exactly as a signed release is.

const LEVELS: &str = "Working/Data/Levels.bin";
const OVERLAY: &str = "Working/Data/Overlay.dat";
const EXTRA: &str = "Working/Data/Extra.dat";
const EXTRA_OUT: &[u8] = b"extra written by patch 1";

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn sha(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes).as_slice())
}

/// The index for the real seed and patches, and the release that signs them.
/// The blobs are `seed.zip`, `patch0.zip` and `patch1.zip`.
fn real_artifacts() -> (FileIndex, VerifiedRelease, Vec<(&'static str, Vec<u8>)>) {
    let seed = zip_of(&[(LEVELS, SEED_LEVELS), (OVERLAY, SEED_OVERLAY)]);
    let patch0 = zip_of(&[(OVERLAY, OVERLAY_PATCHED)]);
    let patch1 = zip_of(&[(EXTRA, EXTRA_OUT)]);
    let index = FileIndex {
        schema_version: 1,
        setup_policy_version: 1,
        seed: SeedLayer {
            sha256: sha(&seed),
            files: vec![file(LEVELS, SEED_LEVELS), file(OVERLAY, SEED_OVERLAY)],
        },
        patches: vec![
            PatchLayer {
                state_key: "overlay".into(),
                sha256: sha(&patch0),
                inputs: vec![],
                outputs: vec![file(OVERLAY, OVERLAY_PATCHED)],
            },
            PatchLayer {
                state_key: "extra".into(),
                sha256: sha(&patch1),
                inputs: vec![],
                outputs: vec![file(EXTRA, EXTRA_OUT)],
            },
        ],
        setup: vec![],
    };
    let body = serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": sha(&seed)},
        "patches": [
            {"id": "overlay", "blob": "patch0.zip", "size": patch0.len(), "sha256": sha(&patch0)},
            {"id": "extra", "blob": "patch1.zip", "size": patch1.len(), "sha256": sha(&patch1)},
        ],
    });
    let body = serde_json::to_vec(&body).unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    let release =
        crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap();
    let blobs = vec![
        ("seed.zip", seed),
        ("patch0.zip", patch0),
        ("patch1.zip", patch1),
    ];
    (index, release, blobs)
}

const STATIC: &str = "Working/Data/Static.dat";
const SEED_STATIC: &[u8] = b"static as the seed ships it";
const PATCHED: &str = "Working/Data/Patched.bin";
/// The seed ships its bundled PAK as `Cache.en-US`; the install places it under
/// `SourceCache.en-us`, and the index lists that placed path.
const SHIPPED_PAK: &str = "Working/SGWGame/Cache.en-US/Cooked.pak";
const PLACED_PAK: &str = "Working/SGWGame/SourceCache.en-us/Cooked.pak";
const SEED_PAK: &[u8] = b"bundled cooked data as the seed ships it";

/// Patch 0 rewrites the overlay. Patch 1 has two inputs, `Levels` and `Static`,
/// both seed files, and writes `Patched.bin`. The seed also carries the bundled
/// PAK, listed at its placed path.
fn two_input_artifacts() -> (FileIndex, VerifiedRelease, Vec<(&'static str, Vec<u8>)>) {
    let seed = zip_of(&[
        (LEVELS, SEED_LEVELS),
        (OVERLAY, SEED_OVERLAY),
        (STATIC, SEED_STATIC),
        (SHIPPED_PAK, SEED_PAK),
    ]);
    let patch0 = zip_of(&[(OVERLAY, OVERLAY_PATCHED)]);
    let patch1 = zip_of(&[(PATCHED, SET_OUTPUT)]);
    let mut index = FileIndex {
        schema_version: 1,
        setup_policy_version: 1,
        seed: SeedLayer {
            sha256: sha(&seed),
            files: vec![
                file(LEVELS, SEED_LEVELS),
                file(OVERLAY, SEED_OVERLAY),
                file(STATIC, SEED_STATIC),
                file(PLACED_PAK, SEED_PAK),
            ],
        },
        patches: vec![
            PatchLayer {
                state_key: "overlay".into(),
                sha256: sha(&patch0),
                inputs: vec![],
                outputs: vec![file(OVERLAY, OVERLAY_PATCHED)],
            },
            PatchLayer {
                state_key: "set".into(),
                sha256: sha(&patch1),
                inputs: vec![],
                outputs: vec![file(PATCHED, SET_OUTPUT)],
            },
        ],
        setup: vec![],
    };
    index.patches[1].inputs = vec![file(LEVELS, SEED_LEVELS), file(STATIC, SEED_STATIC)];
    let body = serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": sha(&seed)},
        "patches": [
            {"id": "overlay", "blob": "patch0.zip", "size": patch0.len(), "sha256": sha(&patch0)},
            {"id": "set", "blob": "patch1.zip", "size": patch1.len(), "sha256": sha(&patch1)},
        ],
    });
    let body = serde_json::to_vec(&body).unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    let release =
        crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap();
    let blobs = vec![
        ("seed.zip", seed),
        ("patch0.zip", patch0),
        ("patch1.zip", patch1),
    ];
    (index, release, blobs)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_folder_input_is_copied_over_the_seed_file_its_patch_reads() {
    let (index, release, blobs) = two_input_artifacts();
    let server = serve(&blobs).await;
    // Levels is edited, so patch 1 cannot read it from the folder and the seed
    // is needed. Static is intact, so patch 1 reads the folder's copy.
    let (folder, scanned, found) = classify_folder(
        &index,
        &[
            (LEVELS, EDITED),
            (OVERLAY, SEED_OVERLAY),
            (STATIC, SEED_STATIC),
            (PLACED_PAK, SEED_PAK),
        ],
    );
    let planned = plan(&index, &found, &scanned);
    assert!(planned.needs_seed);
    assert_eq!(planned.patches, vec![0, 1]);
    assert!(planned
        .local_for
        .get(&1)
        .is_some_and(|inputs| inputs.contains(STATIC)));

    let state = tempfile::tempdir().unwrap();
    let fetched = fetch(
        state.path(),
        &server,
        &release,
        &planned,
        &SeedSourceHandle::download(),
    );
    assert_eq!(
        requests(&server).await,
        ["/patch0.zip", "/patch1.zip", "/seed.zip"]
    );

    let prep = tempfile::tempdir().unwrap();
    let replaced = super::replay::replay(
        &planned,
        &index,
        &release,
        &fetched,
        folder.path(),
        &scanned,
        prep.path(),
        &CancellationToken::new(),
        &sink(),
    )
    .unwrap();
    assert_eq!(replaced.len(), 3);
    assert_eq!(std::fs::read(&replaced[LEVELS]).unwrap(), SEED_LEVELS);
    assert_eq!(std::fs::read(&replaced[OVERLAY]).unwrap(), OVERLAY_PATCHED);
    assert_eq!(std::fs::read(&replaced[PATCHED]).unwrap(), SET_OUTPUT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_replay_places_the_seeds_bundled_cooked_data() {
    let (index, release, blobs) = two_input_artifacts();
    let server = serve(&blobs).await;
    // The bundled PAK is Missing from the folder, so the replay must produce it
    // from the seed's shipped name.
    let (folder, scanned, found) = classify_folder(
        &index,
        &[
            (LEVELS, SEED_LEVELS),
            (OVERLAY, SEED_OVERLAY),
            (STATIC, SEED_STATIC),
        ],
    );
    let planned = plan(&index, &found, &scanned);
    assert!(planned.needs_seed);

    let state = tempfile::tempdir().unwrap();
    let fetched = fetch(
        state.path(),
        &server,
        &release,
        &planned,
        &SeedSourceHandle::download(),
    );
    assert_eq!(
        requests(&server).await,
        ["/patch0.zip", "/patch1.zip", "/seed.zip"]
    );

    let prep = tempfile::tempdir().unwrap();
    let replaced = super::replay::replay(
        &planned,
        &index,
        &release,
        &fetched,
        folder.path(),
        &scanned,
        prep.path(),
        &CancellationToken::new(),
        &sink(),
    )
    .unwrap();
    assert_eq!(std::fs::read(&replaced[PLACED_PAK]).unwrap(), SEED_PAK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_replay_output_the_release_does_not_sign_is_named_as_a_mismatch() {
    let (mut index, release, blobs) = real_artifacts();
    // The index promises an overlay that patch 0 does not write.
    index.patches[0].outputs = vec![file(OVERLAY, b"overlay the release never signed")];
    let server = serve(&blobs).await;
    let (folder, scanned, found) =
        classify_folder(&index, &[(LEVELS, SEED_LEVELS), (OVERLAY, SEED_OVERLAY)]);
    let planned = plan(&index, &found, &scanned);
    let state = tempfile::tempdir().unwrap();
    let fetched = fetch(
        state.path(),
        &server,
        &release,
        &planned,
        &SeedSourceHandle::download(),
    );
    let prep = tempfile::tempdir().unwrap();
    let (result, record) = rejection::scope(|| {
        super::replay::replay(
            &planned,
            &index,
            &release,
            &fetched,
            folder.path(),
            &scanned,
            prep.path(),
            &CancellationToken::new(),
            &sink(),
        )
    });
    assert_eq!(result.err(), Some(Error::InvalidArtifact));
    let record = record.unwrap();
    assert_eq!(record.check, "patch_replay_mismatch");
    assert_eq!(record.entry.as_deref(), Some(OVERLAY));
}

async fn serve(blobs: &[(&str, Vec<u8>)]) -> MockServer {
    let server = MockServer::start().await;
    for (name, bytes) in blobs {
        Mock::given(path(format!("/{name}")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.clone()))
            .mount(&server)
            .await;
    }
    server
}

/// The request paths the origin has seen, sorted.
async fn requests(server: &MockServer) -> Vec<String> {
    let mut paths: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| request.url.path().to_string())
        .collect();
    paths.sort();
    paths
}

/// Blocking fetch of what `planned` needs, as the retained worker runs it.
fn fetch(
    state: &Path,
    server: &MockServer,
    release: &VerifiedRelease,
    planned: &ReplayPlan,
    source: &SeedSourceHandle,
) -> artifacts::Fetched {
    let transport =
        artifacts::Transport::loopback(format!("{}/manifest.json", server.uri())).unwrap();
    let progress = ProgressSink::latest().0;
    let cancel = CancellationToken::new();
    tokio::task::block_in_place(|| {
        artifacts::fetch_some(
            state,
            &transport,
            release,
            planned.needs_seed,
            &planned.patches,
            source,
            &cancel,
            &progress,
        )
    })
    .unwrap()
}

fn sink() -> UnpackSink {
    UnpackSink {
        progress: ProgressSink::latest().0,
        label: "Replay fixture".into(),
        cancel: CancellationToken::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stock_folder_fetches_only_the_patch_blobs_and_replays_to_the_release() {
    let (index, release, blobs) = real_artifacts();
    let server = serve(&blobs).await;
    let (folder, scanned, found) =
        classify_folder(&index, &[(LEVELS, SEED_LEVELS), (OVERLAY, SEED_OVERLAY)]);
    let planned = plan(&index, &found, &scanned);
    assert!(!planned.needs_seed);
    assert_eq!(planned.patches, vec![0, 1]);

    let state = tempfile::tempdir().unwrap();
    let fetched = fetch(
        state.path(),
        &server,
        &release,
        &planned,
        &SeedSourceHandle::download(),
    );
    assert_eq!(requests(&server).await, ["/patch0.zip", "/patch1.zip"]);

    let prep = tempfile::tempdir().unwrap();
    let replaced = super::replay::replay(
        &planned,
        &index,
        &release,
        &fetched,
        folder.path(),
        &scanned,
        prep.path(),
        &CancellationToken::new(),
        &sink(),
    )
    .unwrap();
    assert_eq!(replaced.len(), 2);
    assert_eq!(std::fs::read(&replaced[OVERLAY]).unwrap(), OVERLAY_PATCHED);
    assert_eq!(std::fs::read(&replaced[EXTRA]).unwrap(), EXTRA_OUT);
    assert!(!prep.path().join("scratch").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_modified_overlay_output_fetches_only_its_patch() {
    let (index, release, blobs) = real_artifacts();
    let server = serve(&blobs).await;
    let (folder, scanned, found) = classify_folder(
        &index,
        &[(LEVELS, SEED_LEVELS), (OVERLAY, EDITED), (EXTRA, EXTRA_OUT)],
    );
    let planned = plan(&index, &found, &scanned);
    assert_eq!(planned.patches, vec![0]);

    let state = tempfile::tempdir().unwrap();
    let fetched = fetch(
        state.path(),
        &server,
        &release,
        &planned,
        &SeedSourceHandle::download(),
    );
    assert_eq!(requests(&server).await, ["/patch0.zip"]);

    let prep = tempfile::tempdir().unwrap();
    let replaced = super::replay::replay(
        &planned,
        &index,
        &release,
        &fetched,
        folder.path(),
        &scanned,
        prep.path(),
        &CancellationToken::new(),
        &sink(),
    )
    .unwrap();
    assert_eq!(replaced.len(), 1);
    assert_eq!(std::fs::read(&replaced[OVERLAY]).unwrap(), OVERLAY_PATCHED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_estimate_equals_the_bytes_of_the_fetched_blobs() {
    let (index, release, blobs) = real_artifacts();
    let server = serve(&blobs).await;
    let (_folder, scanned, found) =
        classify_folder(&index, &[(LEVELS, SEED_LEVELS), (OVERLAY, SEED_OVERLAY)]);
    let planned = plan(&index, &found, &scanned);

    let state = tempfile::tempdir().unwrap();
    let fetched = fetch(
        state.path(),
        &server,
        &release,
        &planned,
        &SeedSourceHandle::download(),
    );
    let downloaded = estimate(&planned, release.manifest(), false, |_| false);
    assert!(!downloaded.seed);
    assert_eq!(downloaded.patches.len(), 2);
    let fetched_bytes: u64 = fetched
        .patches
        .iter()
        .flatten()
        .map(|path| std::fs::metadata(path).unwrap().len())
        .sum();
    assert_eq!(downloaded.bytes, fetched_bytes);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_modified_seed_file_with_a_local_seed_never_requests_the_seed() {
    let (index, release, blobs) = real_artifacts();
    let server = serve(&blobs).await;
    let (folder, scanned, found) =
        classify_folder(&index, &[(LEVELS, EDITED), (OVERLAY, SEED_OVERLAY)]);
    let planned = plan(&index, &found, &scanned);
    assert!(planned.needs_seed);

    let archive_dir = tempfile::tempdir().unwrap();
    let archive = archive_dir.path().join("SGW-seed.zip");
    std::fs::write(&archive, &blobs[0].1).unwrap();
    let verified = verify_local_seed(
        &archive,
        &release.manifest().seed,
        &CancellationToken::new(),
        &ProgressSink::latest().0,
    )
    .unwrap();
    let source = SeedSourceHandle::download();
    source.use_local(verified);

    let state = tempfile::tempdir().unwrap();
    let fetched = fetch(state.path(), &server, &release, &planned, &source);
    assert!(fetched.seed.is_some());
    assert_eq!(requests(&server).await, ["/patch0.zip", "/patch1.zip"]);
    assert_eq!(std::fs::read(&archive).unwrap(), blobs[0].1);

    let prep = tempfile::tempdir().unwrap();
    let replaced = super::replay::replay(
        &planned,
        &index,
        &release,
        &fetched,
        folder.path(),
        &scanned,
        prep.path(),
        &CancellationToken::new(),
        &sink(),
    )
    .unwrap();
    assert_eq!(std::fs::read(&replaced[LEVELS]).unwrap(), SEED_LEVELS);
    assert_eq!(std::fs::read(&replaced[OVERLAY]).unwrap(), OVERLAY_PATCHED);
    assert_eq!(std::fs::read(&replaced[EXTRA]).unwrap(), EXTRA_OUT);
}

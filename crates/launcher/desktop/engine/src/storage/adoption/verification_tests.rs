//! Index acquisition against a loopback origin: a published index is used, a
//! refused one falls back to the build, and a verified cache is not refetched.
//! Runs on every OS; the signed fixtures are tiny and built here.
use super::index_source::IndexFallback;
use super::verification::{acquire_index, classify, download_for, IndexOrigin};

/// A refused-index fallback for the origin-only tests.
fn fallback() -> IndexFallback {
    IndexFallback {
        reason: "index_not_published",
        check: None,
        http_status: None,
        seed_bytes: 0,
    }
}
use super::*;
use cimmeria_launcher_core::file_index::FileIndex;
use cimmeria_launcher_core::seed_source::SeedSourceHandle;
use ed25519_dalek::{Signer, SigningKey};
use std::io::{Cursor, Write};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

const INDEX_BLOB: &str = "index/file-index.json";

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
/// A minimal PE32 image: enough headers for `client_setup::aslr` to patch.
fn minimal_pe() -> Vec<u8> {
    let mut exe = vec![0u8; 0x200];
    exe[..2].copy_from_slice(b"MZ");
    exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
    exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
    exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
    exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
    exe[0x186..0x188].copy_from_slice(&0x40u16.to_le_bytes());
    exe
}
fn seed_bytes() -> Vec<u8> {
    let exe = minimal_pe();
    archive(&[
        ("Working/Binaries/SGW.exe", exe.as_slice()),
        ("later.txt", b"later entry"),
    ])
}
/// A release signed with the test key; `file_index` is the manifest's optional field.
pub(super) fn release(seed: &[u8], file_index: Option<serde_json::Value>) -> VerifiedRelease {
    let mut body = serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": hex(&Sha256::digest(seed))},
        "patches": [],
    });
    if let Some(index) = file_index {
        body["file_index"] = index;
    }
    signed(body)
}
/// `body` signed with the test key and verified as a release.
fn signed(body: serde_json::Value) -> VerifiedRelease {
    let body = serde_json::to_vec(&body).unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap()
}
/// The index the build produces for `seed` with no patches, as published bytes.
pub(super) fn published_index(seed: &[u8]) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let seed_path = dir.path().join("seed.zip");
    std::fs::write(&seed_path, seed).unwrap();
    let plain = release(seed, None);
    let sink = crate::unpack::UnpackSink {
        progress: crate::install_progress::ProgressSink::latest().0,
        label: "File index fixture".into(),
        cancel: CancellationToken::new(),
    };
    let index = cimmeria_launcher_core::file_index::build(
        &seed_path,
        &[],
        plain.manifest(),
        &dir.path().join("index"),
        &sink,
    )
    .unwrap();
    serde_json::to_vec(&index).unwrap()
}
/// The manifest's `file_index` for `index`; `sha` overrides the signed digest.
pub(super) fn reference(index: &[u8], sha: Option<String>) -> serde_json::Value {
    serde_json::json!({
        "blob": INDEX_BLOB,
        "size": index.len(),
        "sha256": sha.unwrap_or_else(|| hex(&Sha256::digest(index))),
    })
}
async fn origin(seed: &[u8], index: &[u8]) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed.to_vec()))
        .mount(&server)
        .await;
    Mock::given(path("/index/file-index.json"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(index.to_vec()))
        .mount(&server)
        .await;
    server
}
/// Blocking acquisition on this runtime thread, as the retained worker runs it.
fn acquire(
    state: &Path,
    release: &VerifiedRelease,
    server: &MockServer,
) -> Result<(FileIndex, IndexOrigin), Error> {
    let transport = artifacts::Transport::loopback(format!("{}/manifest.json", server.uri()))?;
    let progress = crate::install_progress::ProgressSink::latest().0;
    tokio::task::block_in_place(|| {
        acquire_index(
            state,
            release,
            &transport,
            &SeedSourceHandle::download(),
            None,
            &CancellationToken::new(),
            &progress,
        )
    })
}
/// The request paths the origin has seen, oldest first.
async fn requests(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| request.url.path().to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_published_index_is_used_without_fetching_the_seed() {
    let seed = seed_bytes();
    let index = published_index(&seed);
    let signed = release(&seed, Some(reference(&index, None)));
    let server = origin(&seed, &index).await;
    let state = tempfile::tempdir().unwrap();
    let (_, origin) = acquire(state.path(), &signed, &server).unwrap();
    assert_eq!(origin, IndexOrigin::Published);
    assert_eq!(requests(&server).await, vec![format!("/{INDEX_BLOB}")]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_published_index_with_the_wrong_digest_falls_back_to_the_build() {
    let seed = seed_bytes();
    let index = published_index(&seed);
    let wrong = hex(&Sha256::digest(b"another index"));
    let signed = release(&seed, Some(reference(&index, Some(wrong.clone()))));
    let server = origin(&seed, &index).await;
    let state = tempfile::tempdir().unwrap();
    let (result, logs) = crate::test_capture::capture(|| acquire(state.path(), &signed, &server));
    let (found, origin) = result.unwrap();
    let IndexOrigin::Built(fallback) = origin else {
        panic!("a digest mismatch must fall back to the build, got {origin:?}");
    };
    assert_eq!(fallback.reason, "index_digest_mismatch");
    assert_eq!(fallback.seed_bytes, signed.manifest().seed.size);
    let events = logs
        .iter()
        .filter(|row| row.field("event") == Some("adoption_index_fallback"))
        .count();
    assert_eq!(events, 1);
    assert_eq!(found.seed.sha256, hex(&Sha256::digest(&seed)));
    let seen = requests(&server).await;
    assert_eq!(seen.iter().filter(|p| *p == "/seed.zip").count(), 1);
    assert!(!state
        .path()
        .join(format!("file-index-{wrong}.json"))
        .exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_manifest_without_a_file_index_is_built_with_index_not_published() {
    let seed = seed_bytes();
    let signed = release(&seed, None);
    let server = origin(&seed, &[]).await;
    let state = tempfile::tempdir().unwrap();
    let (_, origin) = acquire(state.path(), &signed, &server).unwrap();
    let IndexOrigin::Built(fallback) = origin else {
        panic!("a manifest with no file index must build, got {origin:?}");
    };
    assert_eq!(fallback.reason, "index_not_published");
    assert_eq!(fallback.seed_bytes, signed.manifest().seed.size);
    assert!(!requests(&server).await.contains(&format!("/{INDEX_BLOB}")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_404_for_the_signed_index_is_index_download_failed() {
    let seed = seed_bytes();
    let index = published_index(&seed);
    let signed = release(&seed, Some(reference(&index, None)));
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed.clone()))
        .mount(&server)
        .await;
    Mock::given(path(format!("/{INDEX_BLOB}")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let state = tempfile::tempdir().unwrap();
    let (_, origin) = acquire(state.path(), &signed, &server).unwrap();
    let IndexOrigin::Built(fallback) = origin else {
        panic!("a 404 for the signed index must build, got {origin:?}");
    };
    assert_eq!(fallback.reason, "index_download_failed");
    // LT-31 adds the HTTP status of a refused download; until then it is not recorded.
    assert_eq!(fallback.http_status, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_verified_cache_is_read_without_another_index_request() {
    let seed = seed_bytes();
    let index = published_index(&seed);
    let sha = hex(&Sha256::digest(&index));
    let signed = release(&seed, Some(reference(&index, None)));
    let server = origin(&seed, &index).await;
    let state = tempfile::tempdir().unwrap();
    let (_, first) = acquire(state.path(), &signed, &server).unwrap();
    assert_eq!(first, IndexOrigin::Published);
    assert!(state.path().join(format!("file-index-{sha}.json")).exists());
    assert_eq!(requests(&server).await, vec![format!("/{INDEX_BLOB}")]);
    let (_, second) = acquire(state.path(), &signed, &server).unwrap();
    assert_eq!(second, IndexOrigin::Published);
    assert_eq!(requests(&server).await, vec![format!("/{INDEX_BLOB}")]);
}

#[test]
fn a_built_index_estimate_downloads_nothing() {
    let estimate = DownloadEstimate {
        seed: true,
        patches: vec!["overlay".into()],
        bytes: 1100,
        local_seed: false,
    };
    let built = download_for(&IndexOrigin::Built(fallback()), estimate.clone());
    assert_eq!(built.bytes, 0);
    assert!(!built.seed);
    assert!(built.patches.is_empty());
    assert_eq!(
        download_for(&IndexOrigin::Published, estimate.clone()),
        estimate
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_built_index_with_a_chosen_local_archive_never_requests_the_seed() {
    let seed = seed_bytes();
    let signed = release(&seed, None);
    let server = origin(&seed, &[]).await;
    let folder = tempfile::tempdir().unwrap();
    let archive_path = folder.path().join("Seed.zip");
    std::fs::write(&archive_path, &seed).unwrap();
    let before = std::fs::read(&archive_path).unwrap();
    let progress = crate::install_progress::ProgressSink::latest().0;
    let verified = cimmeria_launcher_core::seed_source::verify_local_seed(
        &archive_path,
        &signed.manifest().seed,
        &CancellationToken::new(),
        &progress,
    )
    .unwrap();
    let source = SeedSourceHandle::download();
    source.use_local(verified);

    let state = tempfile::tempdir().unwrap();
    let transport =
        artifacts::Transport::loopback(format!("{}/manifest.json", server.uri())).unwrap();
    let (_, found) = tokio::task::block_in_place(|| {
        acquire_index(
            state.path(),
            &signed,
            &transport,
            &source,
            None,
            &CancellationToken::new(),
            &progress,
        )
    })
    .unwrap();
    assert!(matches!(found, IndexOrigin::Built(_)));
    assert!(!requests(&server).await.contains(&"/seed.zip".to_string()));
    assert_eq!(std::fs::read(&archive_path).unwrap(), before);
}

const EXE: &str = "Working/Binaries/SGW.exe";
const LOGIN: &str = "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua";
const EULA: &str = "Working/SGWGame/Content/eula.lua";
const LATER: &str = "Working/SGWGame/Content/later.txt";
/// Rewritten by the overlay: a patch-touched seed file.
const PATCHED: &str = "Working/SGWGame/Content/patched.txt";
/// Added by the overlay: a patch-added file.
const ADDED: &str = "Working/SGWGame/Content/added.txt";

/// The seed's files in their stock bytes. `SGW.exe` still has ASLR on.
fn stock_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (EXE, minimal_pe()),
        (LOGIN, b"stock login".to_vec()),
        (EULA, b"stock eula".to_vec()),
        (LATER, b"later entry".to_vec()),
        (PATCHED, b"seed text".to_vec()),
    ]
}
/// The index built from the stock seed and one overlay that rewrites `PATCHED`
/// and adds `ADDED`. The artifacts are written under `dir`.
fn built_index(dir: &Path) -> FileIndex {
    built_index_without(dir, &[])
}
/// [`built_index`] from a seed that leaves out the paths in `skip`.
fn built_index_without(dir: &Path, skip: &[&str]) -> FileIndex {
    let files: Vec<_> = stock_files()
        .into_iter()
        .filter(|(path, _)| !skip.contains(path))
        .collect();
    let refs: Vec<(&str, &[u8])> = files.iter().map(|(p, b)| (*p, b.as_slice())).collect();
    let seed = archive(&refs);
    let overlay = archive(&[(PATCHED, b"patched text"), (ADDED, b"added by patch")]);
    let seed_path = dir.join("seed.zip");
    std::fs::write(&seed_path, &seed).unwrap();
    let overlay_path = dir.join("overlay.zip");
    std::fs::write(&overlay_path, &overlay).unwrap();
    let verified = signed(serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": hex(&Sha256::digest(&seed))},
        "patches": [{
            "id": "overlay",
            "blob": "overlay.zip",
            "size": overlay.len(),
            "sha256": hex(&Sha256::digest(&overlay)),
        }],
    }));
    let entry = verified.manifest().patches[0].clone();
    let sink = crate::unpack::UnpackSink {
        progress: crate::install_progress::ProgressSink::latest().0,
        label: "Classification fixture".into(),
        cancel: CancellationToken::new(),
    };
    cimmeria_launcher_core::file_index::build(
        &seed_path,
        &[(entry, overlay_path)],
        verified.manifest(),
        &dir.join("index"),
        &sink,
    )
    .unwrap()
}
/// Writes the stock seed files under `root`, leaving out the paths in `skip`.
fn write_stock(root: &Path, skip: &[&str]) {
    for (path, bytes) in stock_files() {
        if !skip.contains(&path) {
            write_under(root, path, &bytes);
        }
    }
}
fn write_under(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}
/// Scans `root` and classifies it against `index` with the default login servers.
fn classify_tree(root: &Path, index: &FileIndex) -> Vec<Difference> {
    let scanned = inventory::scan(root, &CancellationToken::new()).unwrap();
    let servers = crate::client_setup::login_servers::default_servers();
    classify(&scanned, index, &servers)
}
/// The classification of the difference at release spelling `path`.
fn class_of(found: &[Difference], path: &str) -> Classification {
    found
        .iter()
        .find(|d| d.path == path)
        .unwrap_or_else(|| panic!("no difference for {path}: {found:?}"))
        .classification
}

#[test]
fn a_stock_folder_needs_patching_for_patch_touched_and_added_files() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index(dir.path());
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[]);
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, PATCHED), Classification::NeedsPatching);
    assert_eq!(class_of(&found, ADDED), Classification::NeedsPatching);
    assert!(found
        .iter()
        .all(|d| d.classification != Classification::Modified));
}

#[test]
fn an_aslr_on_setup_file_is_a_known_transform() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index(dir.path());
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[]);
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, EXE), Classification::KnownTransform);
}

#[test]
fn a_seed_file_in_another_case_spelling_is_a_known_transform() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index(dir.path());
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[EULA]);
    write_under(
        folder.path(),
        "Working/SGWGame/Content/EULA.lua",
        b"stock eula",
    );
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, EULA), Classification::KnownTransform);
}

#[test]
fn a_custom_login_file_is_a_known_transform() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index(dir.path());
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[LOGIN]);
    write_under(folder.path(), LOGIN, b"-- my own login servers\n");
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, LOGIN), Classification::KnownTransform);
}

#[test]
fn a_changed_seed_byte_is_modified() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index(dir.path());
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[LATER]);
    write_under(folder.path(), LATER, b"later entrx");
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, LATER), Classification::Modified);
}

#[test]
fn a_deleted_seed_file_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index(dir.path());
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[LATER]);
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, LATER), Classification::Missing);
}

#[test]
fn a_deleted_patch_added_file_needs_patching() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index(dir.path());
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[]);
    write_under(folder.path(), PATCHED, b"patched text");
    write_under(folder.path(), ADDED, b"added by patch");
    std::fs::remove_file(folder.path().join(ADDED)).unwrap();
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, ADDED), Classification::NeedsPatching);
    assert_eq!(class_of(&found, PATCHED), Classification::Matched);
}

#[test]
fn a_file_no_history_names_is_extra() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index(dir.path());
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[]);
    write_under(folder.path(), "Working/SGWGame/Content/notes.txt", b"mine");
    let found = classify_tree(folder.path(), &index);
    assert_eq!(
        class_of(&found, "Working/SGWGame/Content/notes.txt"),
        Classification::Extra
    );
}

#[test]
fn a_setup_written_login_file_the_seed_lacks_is_matched_not_extra() {
    let dir = tempfile::tempdir().unwrap();
    let index = built_index_without(dir.path(), &[LOGIN]);
    let folder = tempfile::tempdir().unwrap();
    write_stock(folder.path(), &[LOGIN]);
    let servers = crate::client_setup::login_servers::default_servers();
    let rendered = crate::client_setup::login_servers::render(&servers);
    write_under(folder.path(), LOGIN, rendered.as_bytes());
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, LOGIN), Classification::Matched);

    // At another spelling it is a transform, reported at the rule's spelling.
    let folded = LOGIN.replace("LoginInternal", "logininternal");
    std::fs::remove_file(folder.path().join(LOGIN)).unwrap();
    write_under(folder.path(), &folded, rendered.as_bytes());
    let found = classify_tree(folder.path(), &index);
    let login = found.iter().find(|d| d.path == LOGIN).unwrap();
    assert_eq!(login.classification, Classification::KnownTransform);
    assert_eq!(login.source_path.as_deref(), Some(folded.as_str()));
    std::fs::remove_file(folder.path().join(&folded)).unwrap();

    // Any other bytes are not what setup writes, so the file stays extra.
    write_under(folder.path(), LOGIN, b"custom login");
    let found = classify_tree(folder.path(), &index);
    assert_eq!(class_of(&found, LOGIN), Classification::Extra);
}

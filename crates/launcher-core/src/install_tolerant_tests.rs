//! Tests for the tolerant install entry points and the retry of incomplete
//! patches (LA-04). The strict install paths must ignore the incomplete list.

use std::io::Write;
use std::path::Path;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::tests::{fake_manifest, sha_hex, unpack_ctx_parts};
use super::*;
use crate::state::{IncompletePatch, SkippedTarget};

const DIALOG: &str = "Working/SGWGame/UI/Dialog.lua";

fn stock_bytes() -> Vec<u8> {
    b"function Dialog.show()\r\nend\r\n".repeat(40)
}

fn patched_bytes() -> Vec<u8> {
    let mut bytes = stock_bytes();
    bytes.extend_from_slice(b"-- ours\r\n");
    bytes
}

/// A patch zip: a recipe that turns the stock dialog into the patched one,
/// plus an overlay `hello.txt` that only a full (not retried) apply extracts.
fn recipe_patch_zip(work: &Path) -> Vec<u8> {
    let (stock, patched, spec_dir) = (work.join("stock"), work.join("patched"), work.join("spec"));
    for root in [&stock, &patched] {
        std::fs::create_dir_all(root.join("Working/SGWGame/UI")).unwrap();
    }
    std::fs::create_dir_all(&spec_dir).unwrap();
    std::fs::write(stock.join(DIALOG), stock_bytes()).unwrap();
    std::fs::write(patched.join(DIALOG), patched_bytes()).unwrap();
    let spec: cimmeria_patchset::Spec = serde_json::from_str(&format!(
        r#"{{"id":"one-op","ops":[{{"target":"{DIALOG}","sources":[{{"path":"{DIALOG}"}}]}}]}}"#
    ))
    .unwrap();
    let built = cimmeria_patchset::build(&spec, &spec_dir, &stock, &patched).unwrap();
    let mut src = zip::ZipArchive::new(std::io::Cursor::new(built.zip)).unwrap();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..src.len() {
        out.raw_copy_file(src.by_index_raw(i).unwrap()).unwrap();
    }
    out.start_file("hello.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    out.write_all(b"hi").unwrap();
    out.finish().unwrap().into_inner()
}

/// A server that serves `zip` as the blob of patch `001`.
async fn server_with_blob(zip: &[u8]) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/p/001.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip.to_vec()))
        .mount(&server)
        .await;
    server
}

/// The manifest with one patch `001` whose blob is `zip`.
fn manifest_for(zip: &[u8]) -> crate::manifest::Manifest {
    let mut manifest = fake_manifest("5eed");
    manifest.patches = vec![PatchEntry {
        id: "001".into(),
        blob: "p/001.zip".into(),
        size: zip.len() as u64,
        sha256: sha_hex(zip),
        after: None,
        root: Default::default(),
        title: None,
        description: None,
    }];
    manifest
}

/// Write the ledger `state` and a dialog that no longer matches the stock file.
fn seed(dir: &Path, state: InstalledState) {
    state.save(dir).unwrap();
    let ui = dir.join("Working/SGWGame/UI");
    std::fs::create_dir_all(&ui).unwrap();
    std::fs::write(dir.join(DIALOG), b"modded").unwrap();
}

fn fresh_ledger() -> InstalledState {
    InstalledState {
        seed_sha256: Some("5eed".into()),
        ..Default::default()
    }
}

async fn run(
    dir: &Path,
    server: &MockServer,
    manifest: &crate::manifest::Manifest,
    tolerant: bool,
) -> Result<(), InstallError> {
    let (tx, _rx) = unpack_ctx_parts();
    let http = reqwest::Client::new();
    let manifest_url = format!("{}/manifest.json", server.uri());
    let ctx = InstallContext {
        manifest_url: &manifest_url,
        install_dir: dir,
        manifest,
        login_servers: &[],
        cancel: CancellationToken::new(),
        progress: tx.into(),
        http: &http,
        seed_source: &SeedSourceHandle::download(),
    };
    if tolerant {
        install_all_tolerant(ctx).await.0
    } else {
        install_all(ctx).await.0
    }
}

fn skipped_dialog() -> Vec<SkippedTarget> {
    vec![SkippedTarget {
        path: DIALOG.into(),
        reason: "source_mismatch".into(),
    }]
}

#[tokio::test]
async fn tolerant_install_records_the_skipped_op_and_keeps_the_overlay() {
    let work = tempfile::tempdir().unwrap();
    let zip = recipe_patch_zip(work.path());
    let server = server_with_blob(&zip).await;
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), fresh_ledger());

    run(dir.path(), &server, &manifest_for(&zip), true)
        .await
        .unwrap();

    let state = InstalledState::load(dir.path());
    assert_eq!(state.applied_patches, vec!["001".to_string()]);
    assert_eq!(
        state.incomplete_patches,
        vec![IncompletePatch {
            state_key: "001".into(),
            skipped: skipped_dialog(),
        }]
    );
    assert!(dir.path().join("hello.txt").is_file());
}

#[tokio::test]
async fn strict_install_fails_on_the_same_input() {
    let work = tempfile::tempdir().unwrap();
    let zip = recipe_patch_zip(work.path());
    let server = server_with_blob(&zip).await;
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), fresh_ledger());

    let result = run(dir.path(), &server, &manifest_for(&zip), false).await;

    assert!(
        matches!(result, Err(InstallError::PatchesFailed(_))),
        "{result:?}"
    );
    let state = InstalledState::load(dir.path());
    assert!(state.incomplete_patches.is_empty());
    assert_eq!(std::fs::read(dir.path().join(DIALOG)).unwrap(), b"modded");
}

#[tokio::test]
async fn retry_clears_the_entry_once_the_source_is_restored() {
    let work = tempfile::tempdir().unwrap();
    let zip = recipe_patch_zip(work.path());
    let server = server_with_blob(&zip).await;
    let manifest = manifest_for(&zip);
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), fresh_ledger());
    run(dir.path(), &server, &manifest, true).await.unwrap();
    assert_eq!(InstalledState::load(dir.path()).incomplete_patches.len(), 1);

    std::fs::write(dir.path().join(DIALOG), stock_bytes()).unwrap();
    run(dir.path(), &server, &manifest, true).await.unwrap();

    let state = InstalledState::load(dir.path());
    assert!(state.incomplete_patches.is_empty());
    assert_eq!(state.applied_patches, vec!["001".to_string()]);
    assert_eq!(
        std::fs::read(dir.path().join(DIALOG)).unwrap(),
        patched_bytes()
    );
}

/// The R-LA 10 guard: strict `install_all` never retries. Over a ledger that
/// lists an incomplete patch it requests no blob and leaves the list alone.
#[tokio::test]
async fn strict_install_ignores_the_incomplete_list() {
    let work = tempfile::tempdir().unwrap();
    let zip = recipe_patch_zip(work.path());
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/p/001.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip.clone()))
        .expect(0)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let incomplete = vec![IncompletePatch {
        state_key: "001".into(),
        skipped: skipped_dialog(),
    }];
    seed(
        dir.path(),
        InstalledState {
            applied_patches: vec!["001".into()],
            incomplete_patches: incomplete.clone(),
            ..fresh_ledger()
        },
    );

    run(dir.path(), &server, &manifest_for(&zip), false)
        .await
        .unwrap();

    assert!(server
        .received_requests()
        .await
        .unwrap_or_default()
        .is_empty());
    assert_eq!(
        InstalledState::load(dir.path()).incomplete_patches,
        incomplete
    );
}

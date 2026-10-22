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

/// An incomplete entry whose patch has left the manifest is kept as it was.
#[tokio::test]
async fn an_entry_whose_patch_left_the_manifest_is_kept() {
    let work = tempfile::tempdir().unwrap();
    let zip = recipe_patch_zip(work.path());
    let server = server_with_blob(&zip).await;
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        InstalledState {
            applied_patches: vec!["001".into(), "gone".into()],
            incomplete_patches: vec![
                IncompletePatch {
                    state_key: "001".into(),
                    skipped: skipped_dialog(),
                },
                IncompletePatch {
                    state_key: "gone".into(),
                    skipped: skipped_dialog(),
                },
            ],
            ..fresh_ledger()
        },
    );

    run(dir.path(), &server, &manifest_for(&zip), true)
        .await
        .unwrap();

    let state = InstalledState::load(dir.path());
    let keys: Vec<&str> = state
        .incomplete_patches
        .iter()
        .map(|i| i.state_key.as_str())
        .collect();
    assert_eq!(keys, vec!["001", "gone"]);
    let gone = state
        .incomplete_patches
        .iter()
        .find(|i| i.state_key == "gone")
        .unwrap();
    assert_eq!(gone.skipped, skipped_dialog());
}

/// A retry whose blob cannot be fetched is logged, not returned: the run
/// succeeds and the incomplete entry stays for the next run.
#[tokio::test]
async fn a_failed_retry_keeps_the_entry_and_the_run_succeeds() {
    let work = tempfile::tempdir().unwrap();
    let zip = recipe_patch_zip(work.path());
    // No mock is mounted, so every request, the blob included, answers 404.
    let server = MockServer::start().await;
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

    let result = run(dir.path(), &server, &manifest_for(&zip), true).await;

    assert!(result.is_ok(), "{result:?}");
    assert_eq!(
        InstalledState::load(dir.path()).incomplete_patches,
        incomplete
    );
}

/// Records every event's level, target and fields; the crate has no subscriber
/// dev-dependency, so this is the smallest `Subscriber` that can be asserted on.
#[derive(Clone, Default)]
struct Rows(std::sync::Arc<std::sync::Mutex<Vec<(tracing::Level, String, Vec<(String, String)>)>>>);

struct Fields<'a>(&'a mut Vec<(String, String)>);

impl tracing::field::Visit for Fields<'_> {
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        self.0.push((f.name().to_owned(), format!("{v:?}")));
    }
}

impl tracing::Subscriber for Rows {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = Vec::new();
        event.record(&mut Fields(&mut fields));
        self.0.lock().unwrap().push((
            *event.metadata().level(),
            event.metadata().target().to_owned(),
            fields,
        ));
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

/// The `patch_retry_failed` row: closed fields only (R5), never the error's text.
#[tokio::test]
async fn a_failed_retry_logs_one_warning_with_the_patch_label_and_closed_code() {
    let work = tempfile::tempdir().unwrap();
    let zip = recipe_patch_zip(work.path());
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        InstalledState {
            applied_patches: vec!["001".into()],
            incomplete_patches: vec![IncompletePatch {
                state_key: "001".into(),
                skipped: skipped_dialog(),
            }],
            ..fresh_ledger()
        },
    );
    let rows = Rows::default();
    let _guard = tracing::subscriber::set_default(rows.clone());
    let result = run(dir.path(), &server, &manifest_for(&zip), true).await;
    assert!(result.is_ok(), "{result:?}");
    let rows = rows.0.lock().unwrap();
    let field = |row: &(tracing::Level, String, Vec<(String, String)>), key: &str| {
        row.2.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    };
    let hits: Vec<_> = rows
        .iter()
        .filter(|r| {
            field(r, "event").as_deref() == Some("\"patch_retry_failed\"")
                || field(r, "event").as_deref() == Some("patch_retry_failed")
        })
        .collect();
    assert_eq!(hits.len(), 1, "{rows:?}");
    assert_eq!(hits[0].0, tracing::Level::WARN);
    assert_eq!(hits[0].1, "launcher.install");
    let patch = field(hits[0], "patch").unwrap();
    assert!(patch.starts_with("patch:"), "{patch}");
    let reason = field(hits[0], "reason").unwrap();
    assert_eq!(reason.trim_matches('"'), "unexpected_status");
}

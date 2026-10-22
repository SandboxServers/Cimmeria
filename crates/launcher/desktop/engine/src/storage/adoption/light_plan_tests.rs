use super::light_plan::{plan, stock_spelling, LightPlan};
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use std::sync::{Mutex, MutexGuard};
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

static SERIAL: Mutex<()> = Mutex::new(());
fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

const OVERLAY: &str = "Working/Data/Overlay.dat";
const EXTRA: &str = "Working/Data/Extra.dat";
const SEED_OVERLAY: &[u8] = b"overlay as the seed ships it";
const OVERLAY_PATCHED: &[u8] = b"overlay rewritten by patch 0";
const EXTRA_OUT: &[u8] = b"extra written by patch 1";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

/// A signed release with two patches: patch 0 rewrites the overlay and patch 1
/// writes an extra file. The seed is signed but never served.
fn release() -> (VerifiedRelease, Vec<(&'static str, Vec<u8>)>) {
    let seed = zip_of(&[(OVERLAY, SEED_OVERLAY)]);
    let patch0 = zip_of(&[(OVERLAY, OVERLAY_PATCHED)]);
    let patch1 = zip_of(&[(EXTRA, EXTRA_OUT)]);
    let body = serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": hex(&Sha256::digest(&seed))},
        "patches": [
            {"id": "overlay", "blob": "patch0.zip", "size": patch0.len(), "sha256": hex(&Sha256::digest(&patch0))},
            {"id": "extra", "blob": "patch1.zip", "size": patch1.len(), "sha256": hex(&Sha256::digest(&patch1))},
        ],
    });
    let body = serde_json::to_vec(&body).unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    let release =
        crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap();
    let blobs = vec![("patch0.zip", patch0), ("patch1.zip", patch1)];
    (release, blobs)
}

/// Writes `files` under a new folder and scans it, as the verification does.
fn folder(files: &[(&str, &[u8])]) -> (tempfile::TempDir, inventory::Index) {
    let dir = tempfile::tempdir().unwrap();
    for (rel, bytes) in files {
        let path = dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
    }
    let scan = inventory::scan(dir.path(), &CancellationToken::new()).unwrap();
    (dir, scan)
}

async fn serve(blobs: &[(&str, Vec<u8>)]) -> MockServer {
    let server = MockServer::start().await;
    for (name, bytes) in blobs {
        Mock::given(matchers::path(format!("/{name}")))
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

fn transport(server: &MockServer) -> artifacts::Transport {
    artifacts::Transport::loopback(format!("{}/manifest.json", server.uri())).unwrap()
}

/// Runs the plan the way the blocking worker does.
fn run(
    transport: &artifacts::Transport,
    release: &VerifiedRelease,
    state: &Path,
    scan: &inventory::Index,
) -> LightPlan {
    let cancel = CancellationToken::new();
    tokio::task::block_in_place(|| plan(transport, release, state, scan, &cancel)).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stock_folder_plans_every_patch_as_pending() {
    let (release, blobs) = release();
    let server = serve(&blobs).await;
    let (_folder, scan) = folder(&[(OVERLAY, SEED_OVERLAY)]);
    let state = tempfile::tempdir().unwrap();
    let planned = run(&transport(&server), &release, state.path(), &scan);
    assert_eq!(planned.pending, vec![0, 1]);
    assert_eq!(planned.zips.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_folder_at_the_last_patch_plans_none() {
    let (release, blobs) = release();
    let server = serve(&blobs).await;
    let (_folder, scan) = folder(&[(OVERLAY, OVERLAY_PATCHED), (EXTRA, EXTRA_OUT)]);
    let state = tempfile::tempdir().unwrap();
    let planned = run(&transport(&server), &release, state.path(), &scan);
    assert!(planned.pending.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_planned_event_carries_the_four_counts() {
    let (release, blobs) = release();
    let server = serve(&blobs).await;
    let transport = transport(&server);
    let (_folder, scan) = folder(&[(OVERLAY, SEED_OVERLAY)]);
    let state = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    // The guard is taken after the last await: one capture test of this module at a time.
    let _serial = serial();
    // A second live Dispatch keeps callsite interest per dispatcher, so a callsite
    // that another module's test hit first is not left disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let (planned, events) = crate::test_capture::capture(|| {
        tracing::callsite::rebuild_interest_cache(); // first line in the closure
        tokio::task::block_in_place(|| plan(&transport, &release, state.path(), &scan, &cancel))
    });
    let planned = planned.unwrap();
    let event = events
        .iter()
        .find(|e| e.field("event") == Some("adoption_light_planned"))
        .expect("the plan emits adoption_light_planned");
    assert_eq!(event.field("patches_total"), Some("2"));
    assert_eq!(event.field("patches_current"), Some("0"));
    assert_eq!(event.field("patches_pending"), Some("2"));
    let bytes = planned.download_bytes.to_string();
    assert_eq!(event.field("download_bytes"), Some(bytes.as_str()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_request_asks_for_the_seed() {
    let (release, blobs) = release();
    let server = serve(&blobs).await;
    let (_folder, scan) = folder(&[(OVERLAY, SEED_OVERLAY)]);
    let state = tempfile::tempdir().unwrap();
    run(&transport(&server), &release, state.path(), &scan);
    assert_eq!(requests(&server).await, ["/patch0.zip", "/patch1.zip"]);
}

#[test]
fn a_placed_bundled_cooked_path_maps_back_to_its_stock_spelling() {
    assert_eq!(
        stock_spelling("Working/SGWGame/SourceCache.en-us/Cooked.pak").as_deref(),
        Some("Working/SGWGame/Cache.en-US/Cooked.pak")
    );
    assert_eq!(stock_spelling("Working/Data/Overlay.dat"), None);
}

//! Index acquisition against a loopback origin: a published index is used, a
//! refused one falls back to the build, and a verified cache is not refetched.
//! Runs on every OS; the signed fixtures are tiny and built here.
use super::verification::{acquire_index, IndexOrigin};
use super::*;
use cimmeria_launcher_core::file_index::FileIndex;
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
fn release(seed: &[u8], file_index: Option<serde_json::Value>) -> VerifiedRelease {
    let mut body = serde_json::json!({
        "schema": 1,
        "seed": {"blob": "seed.zip", "size": seed.len(), "sha256": hex(&Sha256::digest(seed))},
        "patches": [],
    });
    if let Some(index) = file_index {
        body["file_index"] = index;
    }
    let body = serde_json::to_vec(&body).unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap()
}
/// The index the build produces for `seed` with no patches, as published bytes.
fn published_index(seed: &[u8]) -> Vec<u8> {
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
fn reference(index: &[u8], sha: Option<String>) -> serde_json::Value {
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
    let (found, origin) = acquire(state.path(), &signed, &server).unwrap();
    assert_eq!(origin, IndexOrigin::Built);
    assert_eq!(found.seed.sha256, hex(&Sha256::digest(&seed)));
    let seen = requests(&server).await;
    assert_eq!(seen.iter().filter(|p| *p == "/seed.zip").count(), 1);
    assert!(!state
        .path()
        .join(format!("file-index-{wrong}.json"))
        .exists());
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

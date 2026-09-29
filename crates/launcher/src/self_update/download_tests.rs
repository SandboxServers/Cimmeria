use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::self_update::releases::ReleaseAsset;

const TAG: &str = "launcher-20261002-bbbbbbb";
const EXE: &str = "sgw-launcher-launcher-20261002-bbbbbbb.exe";

fn sha_hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// The loopback stub's endpoints: plain http to 127.0.0.1 only.
pub(crate) fn stub_endpoints(server: &MockServer) -> UpdateEndpoints {
    UpdateEndpoints {
        releases_api: format!("{}/releases", server.uri()),
        allowed_hosts: vec!["127.0.0.1".into()],
        require_https: false,
    }
}

fn release(server: &MockServer, size: u64) -> LauncherRelease {
    LauncherRelease {
        tag: TAG.into(),
        published_at: 2600,
        page_url: UpdateEndpoints::release_page(TAG),
        exe: ReleaseAsset {
            name: EXE.into(),
            url: format!("{}/dl/{EXE}", server.uri()),
            size,
        },
        sha256: ReleaseAsset {
            name: format!("{EXE}.sha256"),
            url: format!("{}/dl/{EXE}.sha256", server.uri()),
            size: 100,
        },
    }
}

async fn serve(server: &MockServer, exe: &[u8], sha_file: String) {
    Mock::given(method("GET"))
        .and(path(format!("/dl/{EXE}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(exe.to_vec()))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/dl/{EXE}.sha256")))
        .respond_with(ResponseTemplate::new(200).set_body_string(sha_file))
        .mount(server)
        .await;
}

async fn run(
    server: &MockServer,
    rel: &LauncherRelease,
    dir: &Path,
) -> Result<PathBuf, DownloadError> {
    let endpoints = stub_endpoints(server);
    let http = endpoints.client("sgw-launcher-test").unwrap();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    download_verified(&http, &endpoints, rel, dir, &tx).await
}

#[tokio::test]
async fn a_matching_download_is_verified_and_kept() {
    let server = MockServer::start().await;
    let body = b"new launcher bytes".to_vec();
    serve(&server, &body, format!("{}  {EXE}\n", sha_hex(&body))).await;
    let dir = tempfile::tempdir().unwrap();
    // A stale partial of another release is cleaned up on the way.
    let stale = part_path(dir.path(), "launcher-20261001-aaaaaaa");
    std::fs::write(&stale, b"old").unwrap();

    let got = run(&server, &release(&server, body.len() as u64), dir.path())
        .await
        .unwrap();
    assert_eq!(got, part_path(dir.path(), TAG));
    assert_eq!(std::fs::read(&got).unwrap(), body);
    assert!(!stale.exists());
}

// Bug shape: a tampered or truncated download installed over the running
// exe. It must be refused and deleted before any swap.
#[tokio::test]
async fn a_sha_mismatch_is_rejected_and_deleted() {
    let server = MockServer::start().await;
    let body = b"tampered bytes".to_vec();
    serve(
        &server,
        &body,
        format!("{}  {EXE}\n", sha_hex(b"the real bytes")),
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let err = run(&server, &release(&server, body.len() as u64), dir.path())
        .await
        .unwrap_err();
    assert_eq!(err.reason(), "sha256_mismatch", "{err}");
    assert!(!part_path(dir.path(), TAG).exists());
}

#[tokio::test]
async fn a_size_mismatch_is_rejected_and_deleted() {
    let server = MockServer::start().await;
    let body = b"short".to_vec();
    serve(&server, &body, format!("{}  {EXE}\n", sha_hex(&body))).await;
    let dir = tempfile::tempdir().unwrap();
    let err = run(&server, &release(&server, 999), dir.path())
        .await
        .unwrap_err();
    assert_eq!(err.reason(), "size_mismatch", "{err}");
    assert!(!part_path(dir.path(), TAG).exists());
}

#[tokio::test]
async fn a_checksum_for_another_file_is_not_trusted() {
    let server = MockServer::start().await;
    let body = b"bytes".to_vec();
    serve(&server, &body, format!("{}  other.exe\n", sha_hex(&body))).await;
    let dir = tempfile::tempdir().unwrap();
    let err = run(&server, &release(&server, body.len() as u64), dir.path())
        .await
        .unwrap_err();
    assert_eq!(err.reason(), "bad_checksum_file", "{err}");
    assert!(!part_path(dir.path(), TAG).exists(), "nothing downloaded");
}

#[tokio::test]
async fn an_asset_url_off_the_allow_list_is_refused_before_any_request() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let mut rel = release(&server, 5);
    rel.exe.url = "http://evil.example/x.exe".into();
    let err = run(&server, &rel, dir.path()).await.unwrap_err();
    assert_eq!(err.reason(), "untrusted_url");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn parse_sha256_file_accepts_sha256sum_shapes() {
    let h = "A".repeat(64);
    assert_eq!(
        parse_sha256_file(&format!("{h}  {EXE}\n"), EXE).unwrap(),
        "a".repeat(64)
    );
    assert_eq!(
        parse_sha256_file(&format!("{h} *{EXE}"), EXE).unwrap(),
        "a".repeat(64)
    );
    assert_eq!(parse_sha256_file(&h, EXE).unwrap(), "a".repeat(64));
    assert!(parse_sha256_file("", EXE).is_err());
    assert!(parse_sha256_file("abc  x.exe", EXE).is_err());
    assert!(parse_sha256_file(&format!("{}  {EXE}", "g".repeat(64)), EXE).is_err());
}

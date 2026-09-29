//! The update check against a loopback stub of the GitHub API (wiremock
//! binds 127.0.0.1 only), and the manifest's optional `min_launcher`.

use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::releases::FetchError;
use super::*;

const FIXTURE: &str = include_str!("testdata/releases.json");

fn stub(server: &MockServer) -> UpdateEndpoints {
    UpdateEndpoints {
        releases_api: format!("{}/releases", server.uri()),
        allowed_hosts: vec!["127.0.0.1".into()],
        require_https: false,
    }
}

// Built 2026-10-02 at 12:00 UTC: after aaaaaaa (09:00), before bbbbbbb
// (18:00).
fn build_a() -> LauncherBuild {
    LauncherBuild::from_parts(Some("launcher-20261002-aaaaaaa"), Some("1790942400"))
}

async fn run_check(server: &MockServer, build: &LauncherBuild) -> Result<CheckOutcome, FetchError> {
    let endpoints = stub(server);
    let http = endpoints.client(&user_agent(build)).unwrap();
    check(&http, &endpoints, build).await
}

#[tokio::test]
async fn check_offers_the_newest_launcher_release_and_sends_a_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/releases"))
        .and(header_exists("user-agent"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .expect(1)
        .mount(&server)
        .await;
    let out = run_check(&server, &build_a()).await.unwrap();
    match out.decision {
        UpdateDecision::Available(r) => assert_eq!(r.tag, "launcher-20261002-bbbbbbb"),
        other => panic!("expected an update, got {other:?}"),
    }
    assert_eq!(out.releases.len(), 2);
}

#[tokio::test]
async fn check_on_the_newest_build_is_up_to_date() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/releases"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .mount(&server)
        .await;
    let b = LauncherBuild::from_parts(Some("launcher-20261002-bbbbbbb"), Some("1790960000"));
    let out = run_check(&server, &b).await.unwrap();
    assert_eq!(
        out.decision,
        UpdateDecision::UpToDate {
            newest: "launcher-20261002-bbbbbbb".into()
        }
    );
}

// A development build must not spend the player's API budget, or offer
// anything.
#[tokio::test]
async fn a_dev_build_never_asks_github() {
    let server = MockServer::start().await;
    let out = run_check(&server, &LauncherBuild::dev()).await.unwrap();
    assert_eq!(out.decision, UpdateDecision::DevBuild);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn rate_limiting_is_reported_as_such() {
    for resp in [
        ResponseTemplate::new(403).insert_header("x-ratelimit-remaining", "0"),
        ResponseTemplate::new(429),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/releases"))
            .respond_with(resp)
            .mount(&server)
            .await;
        let err = run_check(&server, &build_a()).await.unwrap_err();
        assert_eq!(err.reason(), "rate_limited", "{err}");
    }
}

#[tokio::test]
async fn an_unreachable_api_is_reported_offline() {
    // Bind and drop a loopback listener: nothing answers on that port.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let endpoints = UpdateEndpoints {
        releases_api: format!("http://127.0.0.1:{port}/releases"),
        allowed_hosts: vec!["127.0.0.1".into()],
        require_https: false,
    };
    let http = endpoints.client("t").unwrap();
    let err = check(&http, &endpoints, &build_a()).await.unwrap_err();
    assert_eq!(err.reason(), "offline", "{err}");
}

#[tokio::test]
async fn a_redirect_off_the_allow_list_is_not_followed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/dl/x.exe.sha256"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "http://evil.invalid/x.sha256"),
        )
        .mount(&server)
        .await;
    let endpoints = stub(&server);
    let http = endpoints.client("t").unwrap();
    let asset = |n: &str| releases::ReleaseAsset {
        name: n.into(),
        url: format!("{}/dl/{n}", server.uri()),
        size: 5,
    };
    let rel = LauncherRelease {
        tag: "launcher-20261002-bbbbbbb".into(),
        published_at: 1,
        page_url: String::new(),
        exe: asset("x.exe"),
        sha256: asset("x.exe.sha256"),
    };
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let err = download::download_verified(&http, &endpoints, &rel, dir.path(), &tx)
        .await
        .unwrap_err();
    assert_eq!(err.reason(), "http_failed", "{err}");
    assert!(err.to_string().contains("does not trust"), "{err}");
}

/// Handoff hooks for an update that must fail before the handoff.
#[cfg(windows)]
struct NoHandoff;

#[cfg(windows)]
impl handoff::HandoffHooks for NoHandoff {
    fn spawn(&mut self, _: &Path, _: &str, _: u32) -> std::io::Result<u32> {
        panic!("a failed update never starts a new launcher")
    }
    fn release_lock(&mut self) -> bool {
        panic!("a failed update keeps the lock")
    }
    fn exit(&mut self) {
        panic!("a failed update keeps running")
    }
}

// A download into a directory the launcher cannot write must stop before
// any request, with the fallback reason the banner turns into a link.
#[cfg(windows)]
#[tokio::test]
async fn an_unwritable_directory_falls_back_to_the_release_page() {
    let server = MockServer::start().await;
    let endpoints = stub(&server);
    let http = endpoints.client("t").unwrap();
    // A path under a regular file can never be created or written.
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("file");
    std::fs::write(&blocker, b"x").unwrap();
    let exe = blocker.join("sub").join("launcher.exe");
    let asset = |n: &str| releases::ReleaseAsset {
        name: n.into(),
        url: format!("{}/dl/{n}", server.uri()),
        size: 5,
    };
    let rel = LauncherRelease {
        tag: "launcher-20261002-bbbbbbb".into(),
        published_at: 1,
        page_url: String::new(),
        exe: asset("x.exe"),
        sha256: asset("x.exe.sha256"),
    };
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let err = apply(
        &http,
        &endpoints,
        &build_a(),
        &rel,
        &exe,
        &tx,
        &mut NoHandoff,
    )
    .await
    .unwrap_err();
    assert_eq!(err.reason(), "dir_not_writable", "{err}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

// The field is optional (docs/client/launcher-guide.md, "Schema
// versioning policy"): manifests without it still parse, and a manifest
// with it round-trips it.
#[test]
fn min_launcher_is_optional_in_the_manifest() {
    let old = r#"{"schema":1,"seed":{"blob":"s.zip","size":1,"sha256":"ab"},"patches":[]}"#;
    let m: crate::manifest::Manifest = serde_json::from_str(old).unwrap();
    assert_eq!(m.min_launcher, None);
    assert!(
        !serde_json::to_string(&m).unwrap().contains("min_launcher"),
        "absent stays absent, so re-signing an old manifest does not change it"
    );

    let new = r#"{"schema":1,"min_launcher":"launcher-20261002-bbbbbbb","seed":{"blob":"s.zip","size":1,"sha256":"ab"}}"#;
    let m: crate::manifest::Manifest = serde_json::from_str(new).unwrap();
    assert_eq!(m.min_launcher.as_deref(), Some("launcher-20261002-bbbbbbb"));
    let gate = version::check_min_launcher(&build_a(), m.min_launcher.as_deref(), &[]);
    // Same day, bbbbbbb's publish time unknown: let through.
    assert!(!gate.blocks());
}

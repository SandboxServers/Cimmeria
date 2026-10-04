use super::*;

fn entry(id: &str, after: Option<&str>) -> PatchEntry {
    PatchEntry {
        id: id.into(),
        blob: "b".into(),
        size: 1,
        sha256: "ab".into(),
        after: after.map(Into::into),
        root: Default::default(),
        title: None,
        description: None,
    }
}

fn failed(id: &str) -> PatchFailure {
    PatchFailure {
        id: id.into(),
        reason: "r".into(),
    }
}

/// An independent patch still applies after an earlier one failed:
/// the Black Market overlay (`after: null`) must not be lost to a
/// dialog-portrait delta that met a hand-edited file.
#[test]
fn an_independent_patch_is_not_blocked_by_an_earlier_failure() {
    let overlay = entry("bm-ui-overlay", None);
    assert_eq!(
        blocked_by_failure(&overlay, &[failed("001-dialog-portraits")]),
        None
    );
}

#[test]
fn a_patch_built_on_a_failed_patch_is_skipped() {
    let child = entry("002", Some("001"));
    assert_eq!(
        blocked_by_failure(&child, &[failed("001")]).as_deref(),
        Some("001")
    );
    assert_eq!(blocked_by_failure(&child, &[failed("003")]), None);
}

#[test]
fn the_failure_message_names_every_failed_patch() {
    let e = InstallError::PatchesFailed(vec![failed("001"), failed("004")]);
    let msg = e.to_string();
    assert!(msg.starts_with("2 patch(es) not applied"), "{msg}");
    assert!(msg.contains("001: r") && msg.contains("004: r"), "{msg}");
}

#[test]
fn hashes_a_known_file() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x");
    std::fs::write(&p, b"hello world").unwrap();
    assert_eq!(
        hash_file(&p).unwrap(),
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn verify_sha256_succeeds_on_match() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x");
    std::fs::write(&p, b"hello world").unwrap();
    verify_sha256(
        &p,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        "test",
    )
    .unwrap();
}

#[test]
fn verify_sha256_fails_on_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x");
    std::fs::write(&p, b"hello world").unwrap();
    let err = verify_sha256(&p, "deadbeef", "test").unwrap_err();
    assert!(matches!(err, InstallError::HashMismatch { .. }));
}

// Regression guard for the non-panicking HTTP-status branch. An
// earlier version called `error_for_status_ref().unwrap_err()` here,
// which panicked on stray 1xx/3xx because that helper only returns
// `Err` for 4xx/5xx; the current code returns
// `InstallError::UnexpectedStatus` carrying the status + url.
#[tokio::test]
async fn download_to_file_returns_unexpected_status_on_non_2xx() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(418))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join(".tmp-test.zip");
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<Progress>();
    let url = format!("{}/seed.zip", server.uri());
    // Test client allows http:// — production worker client is
    // configured with https_only(true).
    let http = reqwest::Client::new();
    let err = download_to_file(&http, &url, &dest, CancellationToken::new(), 0, "seed", &tx)
        .await
        .expect_err("418 must surface as an error, not panic");

    match err {
        InstallError::UnexpectedStatus { status, url: u } => {
            assert_eq!(status, 418);
            assert!(u.ends_with("/seed.zip"));
        }
        other => panic!("expected UnexpectedStatus, got {other:?}"),
    }
}

fn unpack_ctx_parts() -> (
    tokio::sync::mpsc::UnboundedSender<Progress>,
    tokio::sync::mpsc::UnboundedReceiver<Progress>,
) {
    tokio::sync::mpsc::unbounded_channel()
}

// Bug shape: a download that failed its hash was kept, so every retry
// resumed from the bad bytes (or got 416) and failed the same way until
// someone deleted the .tmp file by hand. It must be deleted.
#[tokio::test]
async fn hash_mismatch_deletes_the_download() {
    let dir = tempfile::tempdir().unwrap();
    let tmp = dir.path().join(".tmp-seed-abc.download");
    std::fs::write(&tmp, b"corrupt").unwrap();
    let (tx, _rx) = unpack_ctx_parts();
    let manifest = fake_manifest("00");
    let http = reqwest::Client::new();
    let ctx = InstallContext {
        manifest_url: "https://example.invalid/manifest.json",
        install_dir: dir.path(),
        manifest: &manifest,
        login_servers: &[],
        cancel: CancellationToken::new(),
        progress: tx.into(),
        http: &http,
    };
    let err = verify_and_unpack(&ctx, &tmp, ctx.install_dir, "00", "seed")
        .await
        .unwrap_err();
    assert!(matches!(err, InstallError::HashMismatch { .. }), "{err:?}");
    assert!(!tmp.exists(), "a bad download must not be resumed");
}

// A full-length download left over from a run that stopped before
// verifying gets 416 for its Range request. That is "already
// downloaded", not an error; the hash check decides.
#[tokio::test]
async fn download_to_file_treats_416_on_a_complete_file_as_done() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/seed.rar"))
        .respond_with(ResponseTemplate::new(416))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join(".tmp-seed.download");
    std::fs::write(&dest, b"0123456789").unwrap();
    let (tx, _rx) = unpack_ctx_parts();
    let url = format!("{}/seed.rar", server.uri());
    download_to_file(
        &reqwest::Client::new(),
        &url,
        &dest,
        CancellationToken::new(),
        10,
        "seed",
        &tx,
    )
    .await
    .expect("416 on a complete file is success");
    assert_eq!(std::fs::read(&dest).unwrap(), b"0123456789");

    // A short partial file getting 416 is still an error.
    std::fs::write(&dest, b"01234").unwrap();
    let err = download_to_file(
        &reqwest::Client::new(),
        &url,
        &dest,
        CancellationToken::new(),
        10,
        "seed",
        &tx,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        InstallError::UnexpectedStatus { status: 416, .. }
    ));
}

#[test]
fn safe_sha_prefix_accepts_lower_and_upper_hex() {
    assert_eq!(safe_sha_prefix("abcdef0123456789").unwrap(), "abcdef012345");
    assert_eq!(safe_sha_prefix("ABCDEF0123456789").unwrap(), "ABCDEF012345");
}

#[test]
fn safe_sha_prefix_rejects_path_traversal() {
    // The bug shape: a malformed-but-still-signed manifest with a
    // sha256 of "../foo" would, prior to validation, produce a tmp
    // file path that escapes the install directory.
    for bad in &[
        "../foo",
        "/etc/passwd",
        "..\\evil",
        "0123/4567",
        "abc def",
        "",
        "abcg",
    ] {
        let err = safe_sha_prefix(bad).expect_err(&format!("expected reject for {bad:?}"));
        assert!(matches!(err, InstallError::InvalidSha256(_)));
    }
}

#[test]
fn safe_sha_prefix_short_input_passes() {
    // Anything fewer than 12 chars still passes if every char is hex
    // — the prefix is min(len, 12). A real sha256 is always 64 chars
    // so this only matters for malformed manifests; reject those
    // separately via the all-hex check rather than a length check.
    assert_eq!(safe_sha_prefix("abc").unwrap(), "abc");
}

fn fake_manifest(seed_hash: &str) -> crate::manifest::Manifest {
    crate::manifest::Manifest {
        schema: 1,
        min_launcher: None,
        seed: crate::manifest::SeedEntry {
            blob: "seed/x.zip".into(),
            size: 1,
            sha256: seed_hash.into(),
        },
        patches: vec![],
    }
}

// Happy path: directory has SGW.exe, no prior launcher-installed.json
// → adopt writes the marker file with seed_adopted=true and copies
// the manifest seed hash. Subsequent Install/Update would then apply
// patches on top without re-downloading the seed.
#[test]
fn adopt_existing_install_writes_marker_when_sgw_exe_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SGW.exe"), b"fake-game-binary").unwrap();
    let manifest = fake_manifest("seed-hash-from-manifest");
    let state = adopt_existing_install(dir.path(), &manifest).unwrap();
    assert!(state.seed_adopted);
    assert_eq!(
        state.seed_sha256.as_deref(),
        Some("seed-hash-from-manifest")
    );
    assert!(state.applied_patches.is_empty());
    // Persistence path: load-back must produce the same shape so a
    // restart of the launcher sees the adopted install.
    let loaded = InstalledState::load(dir.path());
    assert!(loaded.seed_adopted);
    assert_eq!(
        loaded.seed_sha256.as_deref(),
        Some("seed-hash-from-manifest")
    );
}

// Empty directory: NoGameExe rejects with a path-naming error so the
// UI can surface "this isn't a game install" instead of silently
// marking an empty directory as adopted.
#[test]
fn adopt_existing_install_rejects_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = fake_manifest("h");
    let err = adopt_existing_install(dir.path(), &manifest).unwrap_err();
    match err {
        AdoptError::NoGameExe(p) => assert_eq!(p, dir.path()),
        other => panic!("expected NoGameExe, got {other:?}"),
    }
}

// SGW.exe as a directory (or junction) must not trick adopt into
// marking a non-install as managed. is_file() is the gate.
#[test]
fn adopt_existing_install_rejects_when_sgw_exe_is_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("SGW.exe")).unwrap();
    let manifest = fake_manifest("h");
    let err = adopt_existing_install(dir.path(), &manifest).unwrap_err();
    assert!(matches!(err, AdoptError::NoGameExe(_)));
}

// Adopt-over-managed: the second call refuses because overwriting
// a real install's state would silently discard the applied-patches
// list. User must delete the marker
// by hand to re-adopt.
#[test]
fn adopt_existing_install_rejects_already_managed_install() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
    let manifest = fake_manifest("h");
    adopt_existing_install(dir.path(), &manifest).unwrap();
    let err = adopt_existing_install(dir.path(), &manifest).unwrap_err();
    assert!(matches!(err, AdoptError::AlreadyManaged(_)));
}

fn zip_with(name: &str, body: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zw.start_file(name, zip::write::SimpleFileOptions::default())
        .unwrap();
    zw.write_all(body).unwrap();
    zw.finish().unwrap().into_inner()
}

fn sha_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The install-result telemetry event needs one outcome per patch. Bug
/// shape (2026-09-29 colo playtest): a failed patch was only in the
/// launcher's local log, so the reason had to be asked for by hand.
/// Every outcome kind, and a failed download's hashes, must be in the
/// report.
#[tokio::test]
async fn install_all_reports_every_patch_outcome() {
    use crate::install_report::{PatchOutcomeKind as K, RunResult};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let good = zip_with("hello.txt", b"hi");
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/p/002.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"corrupt".to_vec()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/p/004.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(good.clone()))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let mut manifest = fake_manifest("5eed");
    let patch = |id: &str, sha: &str, size: u64, after: Option<&str>| PatchEntry {
        blob: format!("p/{id}.zip"),
        sha256: sha.into(),
        size,
        ..entry(id, after)
    };
    let bad_sha = "ab".repeat(32);
    manifest.patches = vec![
        patch("001", "01", 1, None),
        patch("002", &bad_sha, 7, None),
        patch("003", "03", 1, Some("002")),
        patch("004", &sha_hex(&good), good.len() as u64, None),
    ];
    crate::state::InstalledState {
        applied_patches: vec!["001".into()],
        seed_sha256: Some("5eed".into()),
        seed_adopted: false,
    }
    .save(dir.path())
    .unwrap();

    let (tx, _rx) = unpack_ctx_parts();
    let http = reqwest::Client::new();
    let manifest_url = format!("{}/manifest.json", server.uri());
    let ctx = InstallContext {
        manifest_url: &manifest_url,
        install_dir: dir.path(),
        manifest: &manifest,
        login_servers: &[],
        cancel: CancellationToken::new(),
        progress: tx.into(),
        http: &http,
    };
    let (result, report) = install_all(ctx).await;
    assert!(
        matches!(result, Err(InstallError::PatchesFailed(_))),
        "{result:?}"
    );
    assert_eq!(report.result, RunResult::PatchesFailed);
    assert!(!report.seed_applied);
    let outcomes: Vec<(&str, K)> = report
        .patches
        .iter()
        .map(|p| (p.id.as_str(), p.outcome))
        .collect();
    assert_eq!(
        outcomes,
        vec![
            ("001", K::Already),
            ("002", K::Failed),
            ("003", K::SkippedDependency),
            ("004", K::Applied),
        ]
    );
    let ev = report.patches[1].evidence.as_ref().expect("hash evidence");
    assert_eq!(ev.kind, "download");
    assert_eq!(ev.expected_sha256, bad_sha);
    assert_eq!(ev.actual_sha256, sha_hex(b"corrupt"));
    assert!(report.patches[2]
        .reason
        .as_deref()
        .unwrap()
        .contains("builds on 002"));
    assert!(dir.path().join("hello.txt").is_file());
}

/// Exercises the shared pipeline with a stalled desktop progress consumer:
/// HTTP download, SHA-256, extraction, patch overlay, preparation, and no-op retry.
#[tokio::test]
async fn desktop_progress_installs_seed_and_patch_then_reuses_saved_state() {
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };
    let server = MockServer::start().await;
    let mut exe = vec![0u8; 0x200];
    exe[..2].copy_from_slice(b"MZ");
    exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
    exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
    exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
    exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
    exe[0x186..0x188].copy_from_slice(&0x40u16.to_le_bytes());
    let seed = zip_with("Working/Binaries/SGW.exe", &exe);
    let patch = zip_with("fixture.txt", b"patched");
    for (name, body) in [("/seed.zip", seed.clone()), ("/patch.zip", patch.clone())] {
        Mock::given(method("GET"))
            .and(path(name))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .expect(1)
            .mount(&server)
            .await;
    }
    let mut manifest = fake_manifest(&sha_hex(&seed));
    manifest.seed.blob = "seed.zip".into();
    manifest.seed.size = seed.len() as u64;
    let mut item = entry("overlay", None);
    item.blob = "patch.zip".into();
    item.sha256 = sha_hex(&patch);
    item.size = patch.len() as u64;
    manifest.patches.push(item);
    let root = tempfile::tempdir().unwrap();
    let url = format!("{}/manifest.json", server.uri());
    let client = reqwest::Client::new();
    let servers = crate::client_setup::login_servers::default_servers();
    let (progress, receiver) = crate::install_progress::ProgressSink::latest();
    for pass in 0..2 {
        let (result, report) = install_all(InstallContext {
            manifest_url: &url,
            install_dir: root.path(),
            manifest: &manifest,
            login_servers: &servers,
            cancel: CancellationToken::new(),
            progress: progress.clone(),
            http: &client,
        })
        .await;
        result.unwrap();
        assert_eq!(report.seed_applied, pass == 0);
    }
    assert!(
        receiver.borrow().is_some(),
        "stalled observer retains a final progress value"
    );
    assert_eq!(
        std::fs::read(root.path().join("fixture.txt")).unwrap(),
        b"patched"
    );
    let final_exe = std::fs::read(root.path().join("Working/Binaries/SGW.exe")).unwrap();
    assert_eq!(
        final_exe[0x186] & 0x40,
        0,
        "client preparation disables ASLR"
    );
    assert!(crate::client_setup::login_servers::path(root.path()).is_file());
    assert!(InstalledState::load(root.path()).has_applied_patch(&manifest.patches[0]));
    server.verify().await;
}

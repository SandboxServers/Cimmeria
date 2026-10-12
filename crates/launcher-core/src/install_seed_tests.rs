use super::*;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};
struct Extractor {
    calls: AtomicUsize,
    uncertain: bool,
}
impl SeedExtractor for Extractor {
    fn extract<'a>(
        &'a self,
        request: SeedExtraction<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<(), InstallError>> + Send + 'a>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert!(!request.destination.exists(), "helper output must be fresh");
            assert!(!request.archive.starts_with(request.destination));
            assert_eq!(std::fs::read(request.archive).unwrap(), b"verified seed");
            std::fs::create_dir(request.destination)?;
            std::fs::write(request.destination.join("seed.txt"), b"prepared")?;
            if self.uncertain {
                Err(InstallError::SeedExtractionUncertain)
            } else {
                Ok(())
            }
        })
    }
}
async fn exercise(uncertain: bool, bad_hash: bool) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/seed"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"verified seed".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let digest = if bad_hash {
        "ab".repeat(32)
    } else {
        Sha256::digest(b"verified seed")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    };
    let mut manifest = Manifest {
        schema: 1,
        min_launcher: None,
        seed: SeedEntry {
            blob: "seed".into(),
            sha256: digest.clone(),
            size: 13,
        },
        patches: vec![],
        file_index: None,
    };
    if uncertain {
        manifest.patches.push(PatchEntry {
            id: "never-dispatched".into(),
            blob: "patch.zip".into(),
            size: 1,
            sha256: "ab".repeat(32),
            after: None,
            root: Default::default(),
            title: None,
            description: None,
        });
        Mock::given(method("GET"))
            .and(path("/patch.zip"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
    }
    if !uncertain && !bad_hash {
        use std::io::Write;
        let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        archive
            .start_file("patch.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"native overlay").unwrap();
        let bytes = archive.finish().unwrap().into_inner();
        manifest.patches.push(PatchEntry {
            id: "native-overlay".into(),
            blob: "patch.zip".into(),
            size: bytes.len() as u64,
            sha256: Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            after: None,
            root: Default::default(),
            title: None,
            description: None,
        });
        Mock::given(method("GET"))
            .and(path("/patch.zip"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
            .expect(1)
            .mount(&server)
            .await;
    }
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("content");
    let cache = root.path().join("cache");
    let extractor = Extractor {
        calls: AtomicUsize::new(0),
        uncertain,
    };
    let http = reqwest::Client::new();
    let url = format!("{}/manifest.json", server.uri());
    let seed_source = SeedSourceHandle::download();
    let run = || {
        install_all_with_seed_extractor(
            InstallContext {
                manifest_url: &url,
                install_dir: &output,
                manifest: &manifest,
                login_servers: &[],
                cancel: CancellationToken::new(),
                progress: ProgressSink::latest().0,
                http: &http,
                seed_source: &seed_source,
            },
            SeedBackend {
                extractor: &extractor,
                cache_directory: &cache,
            },
        )
    };
    let (result, report) = run().await;
    if bad_hash {
        assert!(matches!(result, Err(InstallError::HashMismatch { .. })));
        assert_eq!(extractor.calls.load(Ordering::SeqCst), 0);
        assert!(!output.exists());
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 0);
    } else if uncertain {
        assert!(matches!(result, Err(InstallError::SeedExtractionUncertain)));
        assert!(!report.seed_applied);
        assert!(!InstalledState::path(&output).exists());
        assert_eq!(
            std::fs::read_dir(&cache).unwrap().count(),
            1,
            "input retained on uncertain result"
        );
        assert_eq!(std::fs::read(output.join("seed.txt")).unwrap(), b"prepared");
    } else {
        result.unwrap();
        assert!(report.seed_applied);
        assert_eq!(
            std::fs::read(output.join("patch.txt")).unwrap(),
            b"native overlay"
        );
        assert_eq!(InstalledState::load(&output).seed_sha256, Some(digest));
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 0);
        let (result, report) = run().await;
        result.unwrap();
        assert!(!report.seed_applied);
        assert_eq!(
            extractor.calls.load(Ordering::SeqCst),
            1,
            "matching ledger bypasses extraction"
        );
    }
    server.verify().await;
}
#[tokio::test]
async fn external_seed_uses_fresh_content_and_saves_ledger_before_reuse() {
    exercise(false, false).await;
}
#[tokio::test]
async fn uncertain_seed_retains_input_and_partial_output_without_ledger() {
    exercise(true, false).await;
}
#[tokio::test]
async fn hash_failure_never_dispatches_external_seed() {
    exercise(false, true).await;
}

#[tokio::test]
async fn complete_authenticated_seed_cache_works_offline_but_corruption_fetches_again() {
    for corrupt in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"verified seed".to_vec()))
            .expect(if corrupt { 1 } else { 0 })
            .mount(&server)
            .await;
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        std::fs::create_dir(&cache).unwrap();
        let hash: String = Sha256::digest(b"verified seed")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let manifest: Manifest = serde_json::from_value(serde_json::json!({
            "schema":1,"seed":{"blob":"seed","size":13,"sha256":hash},"patches":[]
        }))
        .unwrap();
        let archive = cache.join(format!(
            ".tmp-seed-{}.download",
            safe_sha_prefix(&hash).unwrap()
        ));
        std::fs::write(
            &archive,
            if corrupt {
                b"corruptedseed"
            } else {
                b"verified seed"
            },
        )
        .unwrap();
        let extractor = Extractor {
            calls: AtomicUsize::new(0),
            uncertain: true,
        };
        let http = reqwest::Client::new();
        let result = apply(
            &InstallContext {
                manifest_url: &format!("{}/manifest.json", server.uri()),
                install_dir: &root.path().join("stage"),
                manifest: &manifest,
                login_servers: &[],
                cancel: CancellationToken::new(),
                progress: ProgressSink::latest().0,
                http: &http,
                seed_source: &SeedSourceHandle::download(),
            },
            &manifest.seed,
            Some(SeedBackend {
                extractor: &extractor,
                cache_directory: &cache,
            }),
        )
        .await;
        assert!(matches!(result, Err(InstallError::SeedExtractionUncertain)));
        assert_eq!(extractor.calls.load(Ordering::SeqCst), 1);
        let requests = server.received_requests().await.unwrap();
        assert!(requests
            .iter()
            .all(|request| !request.headers.contains_key("range")));
        server.verify().await;
    }
}

/// A seed entry for `bytes`, as the signed manifest would carry it.
fn manifest_for_bytes(bytes: &[u8]) -> Manifest {
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    serde_json::from_value(serde_json::json!({
        "schema":1,"seed":{"blob":"seed","size":bytes.len(),"sha256":hash},"patches":[]
    }))
    .unwrap()
}

/// Serves `body` with a pause after its first `split` bytes, so a test can
/// switch the seed while the download is in its body. Returns the base URL.
fn serve_body_with_pause(body: &'static [u8], split: usize) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut stream = stream;
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body[..split]);
                let _ = stream.flush();
                std::thread::sleep(Duration::from_secs(30));
                let _ = stream.write_all(&body[split..]);
            });
        }
    });
    url
}

#[tokio::test]
async fn a_local_archive_for_another_seed_is_ignored_and_the_seed_downloads() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"verified seed".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let other = root.path().join("other.rar");
    std::fs::write(&other, b"other archive").unwrap();
    let other_manifest = manifest_for_bytes(b"other archive");
    let verified_other = crate::seed_source::verify_local_seed(
        &other,
        &other_manifest.seed,
        &CancellationToken::new(),
        &ProgressSink::latest().0,
    )
    .unwrap();
    let manifest = manifest_for_bytes(b"verified seed");
    let cache = root.path().join("cache");
    std::fs::create_dir(&cache).unwrap();
    let extractor = Extractor {
        calls: AtomicUsize::new(0),
        uncertain: false,
    };
    let http = reqwest::Client::new();
    let seed_source = crate::seed_source::SeedSourceHandle::local(verified_other);
    let result = apply(
        &InstallContext {
            manifest_url: &format!("{}/manifest.json", server.uri()),
            install_dir: &root.path().join("stage"),
            manifest: &manifest,
            login_servers: &[],
            cancel: CancellationToken::new(),
            progress: ProgressSink::latest().0,
            http: &http,
            seed_source: &seed_source,
        },
        &manifest.seed,
        Some(SeedBackend {
            extractor: &extractor,
            cache_directory: &cache,
        }),
    )
    .await;
    result.expect("the seed downloads when the local archive is for another seed");
    assert_eq!(extractor.calls.load(Ordering::SeqCst), 1);
    assert_eq!(std::fs::read(&other).unwrap(), b"other archive");
}

#[tokio::test]
async fn a_switch_during_the_body_removes_the_partial_and_uses_the_local_archive() {
    const BODY: &[u8] = b"verified seed";
    let url = serve_body_with_pause(BODY, 4);
    let root = tempfile::tempdir().unwrap();
    let player = root.path().join("player.rar");
    std::fs::write(&player, BODY).unwrap();
    let manifest = manifest_for_bytes(BODY);
    let verified = crate::seed_source::verify_local_seed(
        &player,
        &manifest.seed,
        &CancellationToken::new(),
        &ProgressSink::latest().0,
    )
    .unwrap();
    let cache = root.path().join("cache");
    std::fs::create_dir(&cache).unwrap();
    let partial = cache.join(format!(
        ".tmp-seed-{}.download",
        safe_sha_prefix(&manifest.seed.sha256).unwrap()
    ));
    let extractor = Extractor {
        calls: AtomicUsize::new(0),
        uncertain: false,
    };
    let http = reqwest::Client::new();
    let seed_source = crate::seed_source::SeedSourceHandle::download();
    let apply_run = async {
        apply(
            &InstallContext {
                manifest_url: &format!("{url}/manifest.json"),
                install_dir: &root.path().join("stage"),
                manifest: &manifest,
                login_servers: &[],
                cancel: CancellationToken::new(),
                progress: ProgressSink::latest().0,
                http: &http,
                seed_source: &seed_source,
            },
            &manifest.seed,
            Some(SeedBackend {
                extractor: &extractor,
                cache_directory: &cache,
            }),
        )
        .await
    };
    // Switch once some body bytes are on disk, while the server is paused.
    let switch = async {
        while std::fs::metadata(&partial).map_or(true, |meta| meta.len() == 0) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        seed_source.use_local(verified);
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(apply_run, switch)
    })
    .await
    .expect("the switch stops the paused download");
    result.expect("the local archive installs after the switch");
    assert_eq!(extractor.calls.load(Ordering::SeqCst), 1);
    assert!(!partial.exists());
    assert!(std::fs::read_dir(&cache).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".tmp-seed-")));
    assert_eq!(std::fs::read(&player).unwrap(), BODY);
}

#[tokio::test]
async fn a_switch_to_another_seeds_archive_does_not_stop_the_download() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"verified seed".to_vec())
                .set_delay(Duration::from_secs(1)),
        )
        .expect(2)
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let other = root.path().join("other.rar");
    std::fs::write(&other, b"other archive").unwrap();
    let other_manifest = manifest_for_bytes(b"other archive");
    let verified_other = crate::seed_source::verify_local_seed(
        &other,
        &other_manifest.seed,
        &CancellationToken::new(),
        &ProgressSink::latest().0,
    )
    .unwrap();
    let manifest = manifest_for_bytes(b"verified seed");
    let cache = root.path().join("cache");
    std::fs::create_dir(&cache).unwrap();
    let extractor = Extractor {
        calls: AtomicUsize::new(0),
        uncertain: false,
    };
    let http = reqwest::Client::new();
    let seed_source = crate::seed_source::SeedSourceHandle::download();
    let apply_run = async {
        apply(
            &InstallContext {
                manifest_url: &format!("{}/manifest.json", server.uri()),
                install_dir: &root.path().join("stage"),
                manifest: &manifest,
                login_servers: &[],
                cancel: CancellationToken::new(),
                progress: ProgressSink::latest().0,
                http: &http,
                seed_source: &seed_source,
            },
            &manifest.seed,
            Some(SeedBackend {
                extractor: &extractor,
                cache_directory: &cache,
            }),
        )
        .await
    };
    // Switch once the download has reached the server, to another seed's archive.
    let switch = async {
        while server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        seed_source.use_local(verified_other);
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(apply_run, switch)
    })
    .await
    .expect("the download is not stopped by another seed's archive");
    result.expect("the download finishes when the switched archive is for another seed");
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
    assert_eq!(extractor.calls.load(Ordering::SeqCst), 1);
}

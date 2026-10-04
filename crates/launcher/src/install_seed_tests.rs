use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
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

use super::*;
use std::{cell::Cell, path::PathBuf};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

struct Missing;
impl Probe for Missing {
    fn core_path(&self) -> Option<PathBuf> {
        None
    }
}
#[derive(Default)]
struct InspectMsi(Cell<u32>);
impl Elevate for InspectMsi {
    fn run_msi(&self, msi: &Path) -> Result<u32, super::super::ElevateError> {
        self.0.set(self.0.get() + 1);
        verify(&std::fs::read(msi).unwrap()).unwrap();
        #[cfg(windows)]
        assert!(
            std::fs::OpenOptions::new().write(true).open(msi).is_err(),
            "deny-write handle held through dispatch"
        );
        Ok(0) // An inert inspector, never a real installer.
    }
}

#[test]
fn forged_same_size_legacy_bundle_never_reaches_elevation_or_scratch() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("scratch");
    let elevate = InspectMsi::default();
    for bytes in [vec![0; BYTES], vec![0; 8]] {
        assert_eq!(
            install_bytes(&scratch, &bytes, &elevate, &Missing),
            Err(PhysxError::PackageMismatch)
        );
    }
    assert_eq!(elevate.0.get(), 0);
    assert!(!scratch.exists());
}

#[tokio::test]
async fn legacy_download_refuses_status_redirect_and_wrong_declared_size() {
    let server = MockServer::start().await;
    for response in [
        ResponseTemplate::new(404),
        ResponseTemplate::new(302).insert_header("Location", format!("{}/redirect", server.uri())),
        ResponseTemplate::new(200).set_body_bytes(vec![0; 12]),
    ] {
        Mock::given(method("GET"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            download(&server.uri()).await,
            Err(PhysxError::DownloadFailed)
        );
        server.verify().await;
        server.reset().await;
    }
}

#[tokio::test]
async fn legacy_download_caps_a_body_without_content_length() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let writer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
            assert!(request.len() <= 8192);
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n")
            .unwrap();
        // Close-delimited response: the stream limit, not Content-Length,
        // must reject this response. The receiver may close early.
        let _ = stream.write_all(&vec![0; BYTES + 1]);
    });
    assert!(
        matches!(download(&url).await, Err(PhysxError::DownloadFailed)),
        "a close-delimited oversized response must be refused"
    );
    writer.join().unwrap();
}

/// Supply a separately downloaded, signature-checked NVIDIA MSI. This verifies
/// actual bytes, scratch/handle lifecycle and outcome mapping without running MSI.
#[test]
#[ignore = "requires the official NVIDIA legacy MSI in CIMMERIA_PHYSX_LEGACY_MSI; never installs it"]
fn authenticated_vendor_bundle_uses_guarded_msi_and_cleans_it_up() {
    let path = std::env::var_os("CIMMERIA_PHYSX_LEGACY_MSI").expect("vendor MSI path");
    let bytes = std::fs::read(path).unwrap();
    verify(&bytes).unwrap();
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("scratch");
    let elevate = InspectMsi::default();
    assert_eq!(
        install_bytes(&scratch, &bytes, &elevate, &Missing).unwrap(),
        InstallResult::NotDetected { exit_code: 0 }
    );
    assert_eq!(elevate.0.get(), 1);
    assert_eq!(std::fs::read_dir(&scratch).unwrap().count(), 0);
    let mut tampered = bytes;
    tampered[512] ^= 1;
    assert_eq!(
        install_bytes(&scratch, &tampered, &elevate, &Missing),
        Err(PhysxError::PackageMismatch)
    );
    assert_eq!(elevate.0.get(), 1);
}

#[tokio::test]
#[ignore = "downloads 20 MB from the official NVIDIA endpoint; never installs it"]
async fn official_vendor_download_matches_the_pinned_identity() {
    verify(&download(URL).await.unwrap()).unwrap();
}

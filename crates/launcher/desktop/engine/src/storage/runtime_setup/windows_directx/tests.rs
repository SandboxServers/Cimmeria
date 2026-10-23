use super::*;
use std::time::Duration;
struct System(PathBuf);
impl Probe for System {
    fn x86_directory(&self) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}
fn dll(path: &Path, machine: u16) {
    let mut bytes = vec![0; 88];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
    bytes[64..68].copy_from_slice(b"PE\0\0");
    bytes[68..70].copy_from_slice(&machine.to_le_bytes());
    bytes[86..88].copy_from_slice(&0x2000u16.to_le_bytes());
    std::fs::write(path, bytes).unwrap();
}
#[test]
fn checks_the_exact_x86_libraries_even_when_modern_directx_is_present() {
    let root = tempfile::tempdir().unwrap();
    let probe = System(root.path().into());
    dll(&root.path().join("d3d12.dll"), 0xaa64);
    assert_eq!(detect(&probe, None).missing, COMPONENTS);
    dll(&root.path().join(COMPONENTS[0]), 0x8664);
    dll(&root.path().join(COMPONENTS[1]), 0xaa64);
    assert_eq!(detect(&probe, None).missing, COMPONENTS);
    for name in COMPONENTS {
        dll(&root.path().join(name), 0x014c);
    }
    assert!(detect(&probe, None).installed());
}
#[test]
fn an_adopted_game_can_supply_x86_libraries_but_wrong_or_corrupt_files_do_not_count() {
    let system = tempfile::tempdir().unwrap();
    let game = tempfile::tempdir().unwrap();
    let probe = System(system.path().into());
    dll(&game.path().join(COMPONENTS[0]), 0x014c);
    std::fs::write(game.path().join(COMPONENTS[1]), b"not a DLL").unwrap();
    assert_eq!(detect(&probe, Some(game.path())).missing, [COMPONENTS[1]]);
    dll(&system.path().join(COMPONENTS[1]), 0x014c);
    assert_eq!(
        detect(&probe, Some(game.path())).missing,
        [COMPONENTS[1]],
        "a broken local DLL shadows the system DLL"
    );
    std::fs::remove_file(game.path().join(COMPONENTS[1])).unwrap();
    assert!(detect(&probe, Some(game.path())).installed());
}
#[test]
fn untrusted_installer_bytes_are_refused_before_any_file_is_staged() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("setup");
    assert_eq!(verify(&vec![0; BYTES]), Err(Error::PackageMismatch));
    assert_eq!(
        stage(&scratch, b"wrong").unwrap_err(),
        Error::PackageMismatch
    );
    assert!(!scratch.exists());
}

#[tokio::test]
async fn vendor_download_checks_status_size_and_digest_before_returning_executable_bytes() {
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    let bytes = b"authenticated fixture";
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Mock::given(path("/good"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
        .mount(&server)
        .await;
    Mock::given(path("/large"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0; bytes.len() + 1]))
        .mount(&server)
        .await;
    Mock::given(path("/redirect"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("Location", format!("{}/good", server.uri())),
        )
        .mount(&server)
        .await;
    assert_eq!(
        download_pinned(&format!("{}/good", server.uri()), bytes.len(), &hash)
            .await
            .unwrap(),
        bytes
    );
    assert_eq!(
        download_pinned(
            &format!("{}/good", server.uri()),
            bytes.len(),
            &"0".repeat(64)
        )
        .await,
        Err(Error::PackageMismatch)
    );
    assert_eq!(
        download_pinned(&format!("{}/large", server.uri()), bytes.len(), &hash).await,
        Err(Error::PackageMismatch)
    );
    assert_eq!(
        download_pinned(&format!("{}/redirect", server.uri()), bytes.len(), &hash).await,
        Err(Error::DownloadFailed)
    );
}

#[test]
fn staged_vendor_payload_is_reverified_and_held_against_write_or_deletion() {
    let root = tempfile::tempdir().unwrap();
    let bytes = b"fixture";
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let (path, guard) = stage_pinned(root.path(), bytes, bytes.len(), &hash, ".exe").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    #[cfg(windows)]
    {
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
        assert!(std::fs::remove_file(&path).is_err());
    }
    drop(guard);
    let saved = path.to_path_buf();
    drop(path);
    assert!(!saved.exists());
}

#[tokio::test]
async fn a_progressing_vendor_download_outlives_the_inactivity_limit_but_a_stall_is_refused() {
    fn serve(stall: bool) -> (String, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n")
                .unwrap();
            if stall {
                std::thread::sleep(Duration::from_millis(400));
                return;
            }
            for byte in b"0123456789abcdef" {
                std::thread::sleep(Duration::from_millis(25));
                if stream.write_all(&[*byte]).is_err() {
                    return;
                }
            }
        });
        (format!("http://{address}/installer"), thread)
    }
    let hash: String = Sha256::digest(b"0123456789abcdef")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let inactivity = Duration::from_millis(150);
    let (url, thread) = serve(false);
    let start = std::time::Instant::now();
    assert_eq!(
        download_with_inactivity_timeout(&url, 16, &hash, inactivity)
            .await
            .unwrap(),
        b"0123456789abcdef"
    );
    assert!(start.elapsed() > inactivity);
    thread.join().unwrap();
    let (url, thread) = serve(true);
    assert_eq!(
        download_with_inactivity_timeout(&url, 16, &hash, inactivity).await,
        Err(Error::DownloadFailed)
    );
    thread.join().unwrap();
}

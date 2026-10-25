use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};
#[test]
fn tree_identity_detects_file_mode_symlink_and_added_entry_changes() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("wine");
    std::fs::write(&file, b"runtime").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    symlink("wine", root.path().join("wineboot")).unwrap();
    let token = CancellationToken::new();
    let original = tree::digest(root.path(), &token).unwrap();
    tree::verify(root.path(), &original, &token).unwrap();
    std::fs::write(&file, b"changed").unwrap();
    assert_eq!(
        tree::verify(root.path(), &original, &token),
        Err(RuntimeError::Verification)
    );
    std::fs::write(&file, b"runtime").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        tree::verify(root.path(), &original, &token),
        Err(RuntimeError::Verification)
    );
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::remove_file(root.path().join("wineboot")).unwrap();
    symlink("other", root.path().join("wineboot")).unwrap();
    assert_eq!(
        tree::verify(root.path(), &original, &token),
        Err(RuntimeError::Verification)
    );
    std::fs::remove_file(root.path().join("wineboot")).unwrap();
    symlink("wine", root.path().join("wineboot")).unwrap();
    std::fs::write(root.path().join("extra"), b"").unwrap();
    assert_eq!(
        tree::verify(root.path(), &original, &token),
        Err(RuntimeError::Verification)
    );
}
#[test]
fn symlinked_root_and_locked_cache_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("cache");
    let owner = lock(&cache).unwrap();
    assert!(matches!(lock(&cache), Err(RuntimeError::Busy)));
    drop(owner);
    assert!(lock(&cache).is_ok());
    let alias = root.path().join("alias");
    symlink(&cache, &alias).unwrap();
    assert!(matches!(lock(&alias), Err(RuntimeError::Io)));
    assert_eq!(
        tree::verify(&alias, TREE, &CancellationToken::new()),
        Err(RuntimeError::Verification)
    );
}
#[test]
fn invalid_archive_does_not_create_published_runtime() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    std::fs::write(&archive, b"invalid").unwrap();
    let final_path = root.path().join(NAME);
    assert_eq!(
        publish(
            root.path(),
            &final_path,
            &archive,
            &CancellationToken::new()
        ),
        Err(RuntimeError::Verification)
    );
    assert!(!final_path.exists());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
#[tokio::test]
async fn early_cancellation_does_not_create_cache_or_start_download() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("cache");
    let token = CancellationToken::new();
    token.cancel();
    assert_eq!(
        prepare(cache.clone(), token, ProgressSink::latest().0).await,
        Err(RuntimeError::Cancelled)
    );
    assert!(!cache.exists());
}
#[tokio::test]
async fn damaged_published_cache_fails_without_replacing_or_downloading() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join(NAME);
    std::fs::create_dir(&cache).unwrap();
    std::fs::write(cache.join("preserve"), b"damaged").unwrap();
    assert_eq!(
        prepare(
            root.path().to_path_buf(),
            CancellationToken::new(),
            ProgressSink::latest().0
        )
        .await,
        Err(RuntimeError::Verification)
    );
    assert_eq!(std::fs::read(cache.join("preserve")).unwrap(), b"damaged");
}
#[test]
#[ignore = "requires the pinned upstream runtime archive; never executes Wine"]
fn authenticated_runtime_archive_extracts_and_matches_full_tree_identity() {
    let archive = PathBuf::from(
        std::env::var_os("CIMMERIA_RUNTIME_ARCHIVE").expect("set pinned archive path"),
    );
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join(NAME);
    publish(
        root.path(),
        &destination,
        &archive,
        &CancellationToken::new(),
    )
    .unwrap();
    tree::verify(&destination, TREE, &CancellationToken::new()).unwrap();
    assert!(destination.join("bin/wine").is_file());
    assert!(destination.join("bin/wineserver").is_file());
}

#[tokio::test]
async fn streaming_download_rejects_chunked_overflow_and_truncation() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for body in ["5\r\n12345\r\n0\r\n\r\n", "2\r\n12\r\n0\r\n\r\n"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/archive", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            socket.write_all(format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{body}").as_bytes()).await.unwrap();
        });
        let root = tempfile::tempdir().unwrap();
        let result = download::fetch(
            &reqwest::Client::new(),
            &url,
            4,
            root.path(),
            &CancellationToken::new(),
            &ProgressSink::latest().0,
        )
        .await;
        assert!(matches!(
            result,
            Err(download::Failure {
                error: RuntimeError::Verification,
                ..
            })
        ));
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        server.await.unwrap();
    }
}
#[tokio::test]
async fn cancel_interrupts_stalled_response_and_removes_partial_archive() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/archive", listener.local_addr().unwrap());
    let (started, wait) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nx\r\n")
            .await
            .unwrap();
        started.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    let root = tempfile::tempdir().unwrap();
    let token = CancellationToken::new();
    let cancel = token.clone();
    let cancelling = tokio::spawn(async move {
        wait.await.unwrap();
        cancel.cancel();
    });
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        download::fetch(
            &reqwest::Client::new(),
            &url,
            4,
            root.path(),
            &token,
            &ProgressSink::latest().0,
        ),
    )
    .await
    .unwrap();
    assert!(matches!(
        result,
        Err(download::Failure {
            error: RuntimeError::Cancelled,
            ..
        })
    ));
    cancelling.await.unwrap();
    server.abort();
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn wrong_declared_length_is_rejected_before_waiting_for_body() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/archive", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n")
            .await
            .unwrap();
        std::future::pending::<()>().await;
    });
    let root = tempfile::tempdir().unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        download::fetch(
            &reqwest::Client::new(),
            &url,
            4,
            root.path(),
            &CancellationToken::new(),
            &ProgressSink::latest().0,
        ),
    )
    .await
    .unwrap();
    server.abort();
    assert!(matches!(
        result,
        Err(download::Failure {
            error: RuntimeError::Verification,
            ..
        })
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

async fn fetch_from(server: &wiremock::MockServer, root: &Path) -> Result<(), download::Failure> {
    download::fetch(
        &reqwest::Client::new(),
        &format!("{}/archive", server.uri()),
        4,
        root,
        &CancellationToken::new(),
        &ProgressSink::latest().0,
    )
    .await
    .map(drop)
}

#[tokio::test]
async fn http_error_status_is_a_network_failure_with_its_status() {
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(path("/archive"))
        .respond_with(ResponseTemplate::new(404).set_body_bytes(b"1234".to_vec()))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        fetch_from(&server, root.path()).await,
        Err(download::Failure {
            error: RuntimeError::Network,
            http_status: Some(404),
        })
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn success_with_wrong_content_length_is_a_verification_failure() {
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(path("/archive"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"12345".to_vec()))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        fetch_from(&server, root.path()).await,
        Err(download::Failure {
            error: RuntimeError::Verification,
            http_status: None,
        })
    );
}

#[test]
fn runtime_errors_map_to_failure_codes() {
    use crate::failure::FailureCode;
    assert_eq!(
        RuntimeError::Network.failure(),
        Some(FailureCode::WineRuntimeDownloadFailed)
    );
    assert_eq!(
        RuntimeError::Verification.failure(),
        Some(FailureCode::WineRuntimeInvalid)
    );
    assert_eq!(
        RuntimeError::Extraction.failure(),
        Some(FailureCode::WineRuntimeInvalid)
    );
    assert_eq!(
        RuntimeError::Busy.failure(),
        Some(FailureCode::WineRuntimeBusy)
    );
    assert_eq!(RuntimeError::Io.failure(), Some(FailureCode::LocalIo));
    assert_eq!(RuntimeError::Cancelled.failure(), None);
}

#[test]
fn runtime_prepare_failed_pins_its_fields() {
    let ((), rows) = crate::test_capture::capture(|| {
        events::runtime_prepare_failed(&RuntimeError::Network, Some(404));
        events::runtime_prepare_failed(&RuntimeError::Extraction, None);
        events::runtime_prepare_failed(&RuntimeError::Cancelled, None);
    });
    assert_eq!(rows.len(), 2, "a cancellation logs nothing");
    for row in &rows {
        assert_eq!(row.target, "launcher.install");
        assert_eq!(row.level, tracing::Level::WARN);
        assert_eq!(row.field("event"), Some("runtime_prepare_failed"));
    }
    assert_eq!(rows[0].field("error"), Some("network"));
    assert_eq!(
        rows[0].field("failure_code"),
        Some("wine_runtime_download_failed")
    );
    assert_eq!(rows[0].field("http_status"), Some("404"));
    assert_eq!(rows[1].field("error"), Some("extraction"));
    assert_eq!(rows[1].field("failure_code"), Some("wine_runtime_invalid"));
    assert_eq!(rows[1].field("http_status"), None);
}

#[test]
fn early_cancelled_prepare_logs_nothing_and_io_failure_logs_once() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (results, rows) = crate::test_capture::capture(|| {
        runtime.block_on(async {
            let cancel = CancellationToken::new();
            cancel.cancel();
            let cancelled = prepare(
                tempfile::tempdir().unwrap().path().join("runtimes"),
                cancel,
                ProgressSink::latest().0,
            )
            .await;
            let relative = prepare(
                PathBuf::from("relative/runtimes"),
                CancellationToken::new(),
                ProgressSink::latest().0,
            )
            .await;
            (cancelled, relative)
        })
    });
    assert_eq!(results.0, Err(RuntimeError::Cancelled));
    assert_eq!(results.1, Err(RuntimeError::Io));
    let failed: Vec<_> = rows
        .iter()
        .filter(|row| row.field("event") == Some("runtime_prepare_failed"))
        .collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].field("error"), Some("io"));
    assert_eq!(failed[0].field("failure_code"), Some("local_io"));
}

//! Signed artifact transport against loopback origins and the real state store.
use super::tests::*;
use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

fn remote(f: &Fixture) -> PreviewRequest {
    let mut request = f.request();
    request.artifacts = None;
    request
}
fn store(f: &Fixture) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(f.root.path().join("state/adoption-artifacts"))
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}
fn operation(f: &Fixture) -> Option<OperationState> {
    f.state
        .lock()
        .unwrap()
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .map(|op| op.state)
}
fn preparations(f: &Fixture) -> usize {
    list_preparations(&f.state.lock().unwrap()).unwrap().len()
}
async fn origin(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}
async fn outcome(f: &Fixture, manifest_url: String) -> Result<Preview, Error> {
    test_support::start_preview(f.state.clone(), remote(f), manifest_url)?
        .wait()
        .await
}
/// A failed fetch must leave nothing behind and a terminal, retryable journal.
fn assert_clean_failure(f: &Fixture, before: &(inventory::Index, inventory::Index)) {
    assert_eq!(store(f), Vec::<String>::new());
    assert_eq!(operation(f), Some(OperationState::Cancelled));
    assert_eq!(preparations(f), 0);
    assert!(!f.destination().exists());
    assert_eq!(&f.source_snapshot(), before);
}

/// One raw loopback connection: `head`, then `chunk` repeated until the client
/// leaves, `limit` body bytes were written, or the fixture is dropped.
struct RawOrigin {
    url: String,
    sent: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl RawOrigin {
    fn new(head: String, chunk: Vec<u8>, limit: u64, pause: Duration) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/manifest.json", listener.local_addr().unwrap());
        let sent = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (counted, stopped) = (sent.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if stopped.load(Ordering::SeqCst) {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_write_timeout(Some(Duration::from_millis(200)))
                .unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 4096];
            let count = stream.read(&mut request).unwrap();
            assert!(request[..count].starts_with(b"GET /seed.zip "));
            stream.write_all(head.as_bytes()).unwrap();
            while !stopped.load(Ordering::SeqCst) && counted.load(Ordering::SeqCst) < limit {
                if stream.write_all(&chunk).is_err() {
                    return;
                }
                counted.fetch_add(chunk.len() as u64, Ordering::SeqCst);
                std::thread::sleep(pause);
            }
            // Keep the response open: an EOF would end the transfer for the client.
            while !stopped.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        Self {
            url,
            sent,
            stop,
            thread: Some(thread),
        }
    }
    /// The signed length is declared, one byte arrives, and the rest never does.
    fn stalled(size: u64) -> Self {
        Self::new(
            format!("HTTP/1.1 200 OK\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n"),
            vec![b'P'],
            1,
            Duration::ZERO,
        )
    }
}
impl Drop for RawOrigin {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            if !std::thread::panicking() {
                thread.join().unwrap();
            }
        }
    }
}
async fn until(what: &str, mut ready: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !ready() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signed_download_is_reused_after_a_dismissed_review_and_removed_after_publication() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let seed = std::fs::read(&f.seed).unwrap();
    let server = origin(ResponseTemplate::new(200).set_body_bytes(seed.clone())).await;
    let url = format!("{}/manifest.json", server.uri());
    let preview = outcome(&f, url.clone()).await.unwrap();
    let sha: String = Sha256::digest(&seed)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(store(&f), vec![format!("{sha}.artifact")]);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    preview.discard();
    assert_eq!(operation(&f), Some(OperationState::Cancelled));
    assert_eq!(preparations(&f), 0);
    // The second review authenticates the stored blob instead of fetching again.
    let preview = outcome(&f, url).await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    let handle = preview.report.preview_handle;
    tokio::task::spawn_blocking(move || {
        confirm(
            preview,
            Uuid::new_v4(),
            handle,
            choices(),
            CancellationToken::new(),
        )
    })
    .await
    .unwrap()
    .unwrap();
    assert!(!f.root.path().join("state/adoption-artifacts").exists());
    assert_eq!(preparations(&f), 0);
    assert!(f
        .destination()
        .join("game/Working/Binaries/SGW.exe")
        .is_file());
    assert_eq!(f.source_snapshot(), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_body_running_past_the_signed_size_is_cut_off_instead_of_filling_the_disk() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    const LIMIT: u64 = 64 * 1024 * 1024;
    let chunk = vec![0x5a; 64 * 1024];
    let mut framed = format!("{:x}\r\n", chunk.len()).into_bytes();
    framed.extend_from_slice(&chunk);
    framed.extend_from_slice(b"\r\n");
    // No declared length, so only the streaming bound can stop this response.
    let server = RawOrigin::new(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".into(),
        framed,
        LIMIT,
        Duration::ZERO,
    );
    assert_eq!(
        outcome(&f, server.url.clone()).await.err(),
        Some(Error::InvalidArtifact)
    );
    assert!(
        server.sent.load(Ordering::SeqCst) < LIMIT,
        "the transfer must stop at the signed size, not drain the response"
    );
    assert_clean_failure(&f, &before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_declared_length_other_than_the_signed_size_is_refused() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let mut body = std::fs::read(&f.seed).unwrap();
    body.push(0);
    let server = origin(ResponseTemplate::new(200).set_body_bytes(body)).await;
    assert_eq!(
        outcome(&f, format!("{}/manifest.json", server.uri()))
            .await
            .err(),
        Some(Error::InvalidArtifact)
    );
    assert_clean_failure(&f, &before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn substituted_bytes_of_the_signed_size_are_rejected_and_not_kept() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let mut body = std::fs::read(&f.seed).unwrap();
    let last = body.len() - 1;
    body[last] ^= 1;
    let server = origin(ResponseTemplate::new(200).set_body_bytes(body)).await;
    assert_eq!(
        outcome(&f, format!("{}/manifest.json", server.uri()))
            .await
            .err(),
        Some(Error::InvalidArtifact)
    );
    assert_clean_failure(&f, &before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unavailable_origin_is_a_network_failure_and_a_retry_can_succeed() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let server = origin(ResponseTemplate::new(404)).await;
    assert_eq!(
        outcome(&f, format!("{}/manifest.json", server.uri()))
            .await
            .err(),
        Some(Error::Network)
    );
    assert_clean_failure(&f, &before);
    let seed = std::fs::read(&f.seed).unwrap();
    let server = origin(ResponseTemplate::new(200).set_body_bytes(seed)).await;
    assert!(outcome(&f, format!("{}/manifest.json", server.uri()))
        .await
        .is_ok());
}

#[tokio::test]
async fn the_test_transport_refuses_any_origin_that_is_not_loopback() {
    let f = Fixture::new();
    for url in [
        "https://example.invalid/manifest.json",
        "http://192.0.2.1/manifest.json",
        "not a url",
    ] {
        assert!(matches!(
            test_support::start_preview(f.state.clone(), remote(&f), url.into()),
            Err(Error::Storage(StorageError::InvalidDirectory))
        ));
    }
    assert_eq!(operation(&f), None);
    assert_eq!(store(&f), Vec::<String>::new());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_download_settles_its_owner_even_while_a_reader_holds_the_state() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let size = std::fs::metadata(&f.seed).unwrap().len();
    let server = RawOrigin::stalled(size);
    let mut worker =
        test_support::start_preview(f.state.clone(), remote(&f), server.url.clone()).unwrap();
    until("the first downloaded byte", || {
        store(&f).iter().any(|name| name.ends_with(".download"))
    })
    .await;
    assert_eq!(operation(&f), Some(OperationState::Running));
    // A status reader owns the guard at the moment the worker gives up. The
    // worker must wait for it rather than leave a Running owner behind.
    let reader = f.state.lock().unwrap();
    worker.request_cancel();
    std::thread::sleep(Duration::from_millis(300));
    drop(reader);
    assert_eq!(worker.wait().await.err(), Some(Error::Cancelled));
    assert_clean_failure(&f, &before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_the_preview_worker_stops_its_download_and_releases_ownership() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let size = std::fs::metadata(&f.seed).unwrap().len();
    let server = RawOrigin::stalled(size);
    let worker =
        test_support::start_preview(f.state.clone(), remote(&f), server.url.clone()).unwrap();
    until("the first downloaded byte", || {
        store(&f).iter().any(|name| name.ends_with(".download"))
    })
    .await;
    drop(worker);
    until("the abandoned preparation to end", || {
        operation(&f) == Some(OperationState::Cancelled)
    })
    .await;
    assert_clean_failure(&f, &before);
}

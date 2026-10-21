use super::*;
use crate::test_capture::{capture, Captured};
use ed25519_dalek::{Signer, SigningKey};
use std::io::{Read, Write};

fn signed(body: &[u8]) -> Vec<u8> {
    SigningKey::from_bytes(&[0x2a; 32])
        .sign(body)
        .to_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .into_bytes()
}
const BODY: &[u8] = br#"{"schema":1,"seed":{"blob":"s","size":1,"sha256":"h"},"patches":[{"id":"first","blob":"a","size":1,"sha256":"h","title":"Fix","description":"Details"},{"id":"second","blob":"b","size":1,"sha256":"h","after":"first"}]}"#;

#[test]
fn signed_notes_preserve_order_and_fallback_to_id() {
    let result = notes(decode_verified(BODY, &signed(BODY)).unwrap());
    assert_eq!(result.patches[0].title, "Fix");
    assert_eq!(result.patches[0].description.as_deref(), Some("Details"));
    assert_eq!(result.patches[1].title, "second");
}

#[test]
fn no_unsigned_tampered_or_invalid_catalog_is_displayable() {
    assert!(matches!(
        decode_verified(b"tampered", &signed(BODY)),
        Err(CatalogError::Signature)
    ));
    for signature in [vec![], "é".repeat(64).into_bytes(), vec![b'0'; 128]] {
        assert!(matches!(
            decode_verified(BODY, &signature),
            Err(CatalogError::Signature)
        ));
    }
    for body in [
        b"not json".as_slice(),
        br#"{"schema":99,"seed":{"blob":"s","size":1,"sha256":"h"}}"#,
    ] {
        assert!(matches!(
            decode_verified(body, &signed(body)),
            Err(CatalogError::InvalidManifest)
        ));
    }
    assert!(matches!(
        decode_verified(&vec![0; MAX_BODY + 1], b""),
        Err(CatalogError::TooLarge)
    ));
}

#[tokio::test]
async fn transport_bounds_declared_and_chunked_bodies_and_rejects_http_errors() {
    // A scoped subscriber keeps the shared release_fetch_failed callsite from being
    // cached as "never" when this test's thread races the capture test.
    let _subscriber = tracing::subscriber::set_default(tracing_subscriber::Registry::default());
    for (response, expected) in [
        ("HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n", CatalogError::TooLarge),
        ("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n3\r\ndef\r\n0\r\n\r\n", CatalogError::TooLarge),
        ("HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\n\r\n", CatalogError::Network),
    ] {
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/manifest.json", server.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let _ = socket.read(&mut [0;4096]);
            let _ = socket.write_all(response.as_bytes());
        });
        let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(3)).build().unwrap();
        assert_eq!(read_bounded(&client, &url, 5).await.unwrap_err(), expected);
        worker.join().unwrap();
    }
}

#[test]
fn release_fetch_failed_reports_network_reason_for_closed_port() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let url = format!("http://127.0.0.1:{port}/manifest.json");
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let (result, captured) = crate::test_capture::capture(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(read_bounded(&client, &url, 5))
    });
    assert_eq!(result.unwrap_err(), CatalogError::Network);
    let failed: Vec<_> = captured
        .iter()
        .filter(|row| row.field("event") == Some("release_fetch_failed"))
        .collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].target, "launcher.game_update");
    assert_eq!(failed[0].level, tracing::Level::WARN);
    assert_eq!(failed[0].field("reason"), Some("network"));
    assert_eq!(failed[0].field("http_status"), None);
}

/// The `release_fetch_failed` rows of a capture.
fn fetch_failures(captured: &[Captured]) -> Vec<&Captured> {
    captured
        .iter()
        .filter(|row| row.field("event") == Some("release_fetch_failed"))
        .collect()
}

/// Serves one canned response on a loopback port; the worker ends after one request.
fn serve_once(response: &'static str) -> (String, std::thread::JoinHandle<()>) {
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/manifest.json", server.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let (mut socket, _) = server.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let _ = socket.read(&mut [0; 4096]);
        let _ = socket.write_all(response.as_bytes());
    });
    (url, worker)
}

#[test]
fn fetch_failure_rows_name_the_http_status_and_the_declared_body_size() {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    for (response, expected, reason, http_status) in [
        (
            "HTTP/1.1 500 Server Error\r\nContent-Length: 0\r\n\r\n",
            CatalogError::Network,
            "http_status",
            Some("500"),
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n",
            CatalogError::TooLarge,
            "body",
            None,
        ),
    ] {
        let (url, worker) = serve_once(response);
        let (result, captured) = capture(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(read_bounded(&client, &url, 5))
        });
        worker.join().unwrap();
        assert_eq!(result.unwrap_err(), expected);
        let failed = fetch_failures(&captured);
        assert_eq!(failed.len(), 1, "{captured:?}");
        assert_eq!(failed[0].field("reason"), Some(reason));
        assert_eq!(failed[0].field("http_status"), http_status);
    }
}

#[test]
fn release_fetched_row_carries_the_fetch_duration() {
    let ((), captured) = capture(|| events::release_fetched(Duration::from_millis(9)));
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].target, "launcher.game_update");
    assert_eq!(captured[0].level, tracing::Level::INFO);
    assert_eq!(captured[0].field("event"), Some("release_fetched"));
    assert_eq!(captured[0].field("duration_ms"), Some("9"));
}

#[test]
fn fetch_verification_reports_one_signature_row_but_stored_verification_reports_none() {
    let (result, captured) = capture(|| verify_fetched(b"tampered", &signed(BODY)));
    assert!(result.is_err());
    let failed = fetch_failures(&captured);
    assert_eq!(failed.len(), 1, "{captured:?}");
    assert_eq!(failed[0].field("reason"), Some("signature"));

    let (result, captured) = capture(|| verify_release(b"tampered", &signed(BODY)));
    assert!(result.is_err());
    assert!(fetch_failures(&captured).is_empty(), "{captured:?}");
}

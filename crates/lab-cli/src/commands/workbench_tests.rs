use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

use crate::daemon::status::Endpoint;

use super::{open_with, opener_url};

#[test]
fn opener_uses_bearer_daemon_and_sends_only_local_url_to_browser() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sent, received) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            request.extend_from_slice(&buffer[..count]);
            if request.windows(4).any(|bytes| bytes == b"\r\n\r\n")
                && request
                    .windows(b"\"instance\":\"p2\"".len())
                    .any(|bytes| bytes == b"\"instance\":\"p2\"")
            {
                break;
            }
        }
        sent.send(String::from_utf8(request).unwrap()).unwrap();
        let nonce = "a".repeat(64);
        let body =
            format!("{{\"path\":\"/workbench/session/redeem/{nonce}\",\"instance\":\"p2\"}}");
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let endpoint = Endpoint {
        base_url: format!("http://{address}"),
        token: Some("fixture-bearer".into()),
    };
    let mut opened = String::new();
    open_with(&endpoint, "p2", |url| {
        opened = url.to_owned();
        Ok(())
    })
    .unwrap();
    worker.join().unwrap();
    let request = received.recv().unwrap();
    assert!(request.starts_with("POST /workbench/session/open HTTP/1.1"));
    assert!(request
        .to_ascii_lowercase()
        .contains("authorization: bearer fixture-bearer"));
    assert!(request.contains("\"instance\":\"p2\""));
    assert_eq!(
        opened,
        format!(
            "http://{address}/workbench/session/redeem/{}",
            "a".repeat(64)
        )
    );
    assert!(!opened.contains("fixture-bearer"));
}

#[test]
fn opener_never_sends_bearer_to_non_loopback_endpoint() {
    let endpoint = Endpoint {
        base_url: "http://example.org:8779".into(),
        token: Some("fixture-bearer".into()),
    };
    assert!(opener_url(&endpoint, "default").is_err());
}

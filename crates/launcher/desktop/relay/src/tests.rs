//! Tests for the relay and the callback listener. Upstreams are `wiremock` servers on
//! `127.0.0.1`, which only the `cfg(test)` exemption admits.

use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use url::Url;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::callback::{CallbackResult, CallbackServer};
use crate::{check_upstream, start, RelayConfig, RelayError};

fn config(upstream: &str) -> RelayConfig {
    RelayConfig {
        upstream: Url::parse(upstream).expect("test upstream URL"),
        extra_root_pem: None,
        session: None,
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("test HTTP client")
}

#[tokio::test]
async fn relays_userauth_and_returns_status_body_and_cookie() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/SGWLogin/UserAuth"))
        .and(header("SOAPAction", "\"UserAuth\""))
        .and(header("Content-Type", "text/xml"))
        .and(header("Cookie", "sid=abc"))
        .and(body_string("<soap>req</soap>"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/xml")
                .insert_header("Set-Cookie", "sid=xyz; Path=/")
                .set_body_string("<soap>ok</soap>"),
        )
        .expect(1)
        .mount(&upstream)
        .await;
    let relay = start(config(&upstream.uri())).await.expect("relay starts");

    let resp = client()
        .post(format!("{}/SGWLogin/UserAuth", relay.soap_url()))
        .header("Content-Type", "text/xml")
        .header("SOAPAction", "\"UserAuth\"")
        .header("Cookie", "sid=abc")
        .body("<soap>req</soap>")
        .send()
        .await
        .expect("relay answers");

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["set-cookie"], "sid=xyz; Path=/");
    assert_eq!(resp.text().await.expect("body"), "<soap>ok</soap>");
    relay.shutdown().await;
}

#[tokio::test]
async fn refuses_other_paths_hosts_origins_and_big_bodies() {
    // The upstream is never reached: every case is refused before forwarding.
    let relay = start(config("http://127.0.0.1:9"))
        .await
        .expect("relay starts");
    let base = relay.soap_url();
    let c = client();

    let get = c
        .get(format!("{base}/SGWLogin/UserAuth"))
        .send()
        .await
        .expect("answer");
    assert_eq!(get.status(), 404);
    let other = c.post(format!("{base}/x")).send().await.expect("answer");
    assert_eq!(other.status(), 404);
    let host = c
        .post(format!("{base}/SGWLogin/UserAuth"))
        .header("Host", "evil.example")
        .send()
        .await
        .expect("answer");
    assert_eq!(host.status(), 421);
    let origin = c
        .post(format!("{base}/SGWLogin/UserAuth"))
        .header("Origin", "https://example.org")
        .send()
        .await
        .expect("answer");
    assert_eq!(origin.status(), 403);
    let big = c
        .post(format!("{base}/SGWLogin/UserAuth"))
        .body(vec![b'x'; 65 * 1024])
        .send()
        .await
        .expect("answer");
    assert_eq!(big.status(), 413);
    relay.shutdown().await;
}

#[tokio::test]
async fn upstream_failure_is_502_with_empty_body() {
    // Reserve a loopback port and close it, so the upstream is down.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve port");
    let port = closed.local_addr().expect("local addr").port();
    drop(closed);
    let relay = start(config(&format!("http://127.0.0.1:{port}")))
        .await
        .expect("relay starts");

    let resp = client()
        .post(format!("{}/SGWLogin/UserAuth", relay.soap_url()))
        .body("request")
        .send()
        .await
        .expect("relay answers");

    assert_eq!(resp.status(), 502);
    assert!(resp.bytes().await.expect("body").is_empty());
    relay.shutdown().await;
}

#[tokio::test]
async fn refuses_non_https_upstream_outside_tests() {
    let http = Url::parse("http://example.org").expect("URL");
    assert!(matches!(
        check_upstream(&http, false),
        Err(RelayError::Upstream(_))
    ));
    let with_path = Url::parse("https://example.org/x").expect("URL");
    assert!(matches!(
        check_upstream(&with_path, false),
        Err(RelayError::Upstream(_))
    ));
    let bare = Url::parse("https://example.org").expect("URL");
    assert!(check_upstream(&bare, false).is_ok());
    assert!(matches!(
        start(config("http://example.org")).await,
        Err(RelayError::Upstream(_))
    ));
}

#[tokio::test]
async fn binds_loopback_only() {
    let relay = start(config("http://127.0.0.1:9"))
        .await
        .expect("relay starts");
    assert!(relay.soap_url().starts_with("http://127.0.0.1:"));
    assert_eq!(relay.addr.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_ne!(relay.soap_port(), 0);
    relay.shutdown().await;
}

#[tokio::test(start_paused = true)]
async fn idle_connection_is_closed() {
    let relay = start(config("http://127.0.0.1:9"))
        .await
        .expect("relay starts");
    let mut stream = TcpStream::connect(("127.0.0.1", relay.soap_port()))
        .await
        .expect("connect to relay");
    // Send nothing. The relay must close the socket, so the read sees end of file. The
    // outer timeout only stops a broken relay from hanging the test.
    let mut buf = [0u8; 1];
    let read = tokio::time::timeout(Duration::from_secs(300), stream.read(&mut buf)).await;
    assert!(matches!(read, Ok(Ok(0))), "read: {read:?}");
    relay.shutdown().await;
}

#[tokio::test]
async fn oversized_upstream_reply_is_502() {
    let big = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 256 * 1024 + 1]))
        .mount(&big)
        .await;
    let relay = start(config(&big.uri())).await.expect("relay starts");
    let resp = client()
        .post(format!("{}/SGWLogin/UserAuth", relay.soap_url()))
        .body("request")
        .send()
        .await
        .expect("relay answers");
    assert_eq!(resp.status(), 502);
    assert!(resp.bytes().await.expect("body").is_empty());
    relay.shutdown().await;

    let exact = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 256 * 1024]))
        .mount(&exact)
        .await;
    let relay = start(config(&exact.uri())).await.expect("relay starts");
    let resp = client()
        .post(format!("{}/SGWLogin/UserAuth", relay.soap_url()))
        .body("request")
        .send()
        .await
        .expect("relay answers");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.bytes().await.expect("body").len(), 256 * 1024);
    relay.shutdown().await;
}

/// A `tracing` writer that keeps every byte in memory.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("log buffer").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The capture behind the process-wide subscriber. A global default, not a thread-local one,
/// keeps the callsite interest stable while the other tests run in parallel.
static CAPTURED: OnceLock<Captured> = OnceLock::new();

#[tokio::test]
async fn relay_logs_no_body() {
    let captured = CAPTURED
        .get_or_init(|| {
            let captured = Captured::default();
            let writer = captured.clone();
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::TRACE)
                .with_writer(move || writer.clone())
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("no other global subscriber in this test binary");
            captured
        })
        .clone();
    captured.0.lock().expect("log buffer").clear();

    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("SECRETMARKER reply"))
        .mount(&upstream)
        .await;
    let relay = start(config(&upstream.uri())).await.expect("relay starts");
    let resp = client()
        .post(format!("{}/SGWLogin/UserAuth", relay.soap_url()))
        .header("Cookie", "sid=SECRETMARKER")
        .body("SECRETMARKER request")
        .send()
        .await
        .expect("relay answers");
    assert_eq!(resp.status(), 200);
    relay.shutdown().await;

    let logs =
        String::from_utf8(captured.0.lock().expect("log buffer").clone()).expect("UTF-8 logs");
    assert!(
        logs.contains("launcher.relay.stopped"),
        "logs: {}",
        logs.replace('\n', " | ")
    );
    assert!(!logs.contains("SECRETMARKER"), "logs: {logs}");
}

#[tokio::test]
async fn callback_returns_the_code_for_the_right_state() {
    let cb = CallbackServer::bind().await.expect("bind");
    let uri = cb.redirect_uri();
    assert!(uri.starts_with("http://127.0.0.1:") && uri.ends_with("/cb"));
    assert_eq!(cb.addr.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    let wait = tokio::spawn(cb.wait("state-1", Duration::from_secs(5)));

    let resp = client()
        .get(format!("{uri}?code=abc&state=state-1"))
        .send()
        .await
        .expect("callback answers");

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["cache-control"], "no-store");
    let page = resp.text().await.expect("page");
    assert!(page.contains("You can close this tab"));
    assert_eq!(
        wait.await.expect("wait task"),
        CallbackResult::Code("abc".to_owned())
    );
}

#[tokio::test]
async fn callback_state_mismatch_and_denied_and_timeout() {
    for (query, expected) in [
        ("code=abc&state=wrong", CallbackResult::StateMismatch),
        ("error=access_denied&state=s", CallbackResult::Denied),
        ("code=&state=s", CallbackResult::Denied),
    ] {
        let cb = CallbackServer::bind().await.expect("bind");
        let uri = cb.redirect_uri();
        let wait = tokio::spawn(cb.wait("s", Duration::from_secs(5)));
        let resp = client()
            .get(format!("{uri}?{query}"))
            .send()
            .await
            .expect("callback answers");
        assert_eq!(
            resp.status(),
            if matches!(expected, CallbackResult::StateMismatch) {
                400
            } else {
                200
            }
        );
        assert_eq!(wait.await.expect("wait task"), expected, "query {query}");
    }

    let cb = CallbackServer::bind().await.expect("bind");
    let result = cb.wait("s", Duration::from_millis(100)).await;
    assert_eq!(result, CallbackResult::TimedOut);
}

#[tokio::test(start_paused = true)]
async fn callback_closes_idle_connections() {
    let cb = CallbackServer::bind().await.expect("bind");
    let addr = cb.addr;
    let _wait = tokio::spawn(cb.wait("s", Duration::from_secs(300)));
    let mut stream = TcpStream::connect(addr).await.expect("connect to callback");
    // Send nothing. The listener must close the socket after its connection timeout. The
    // outer timeout only stops a broken listener from hanging the test.
    let mut buf = [0u8; 1];
    let read = tokio::time::timeout(Duration::from_secs(60), stream.read(&mut buf)).await;
    assert!(matches!(read, Ok(Ok(0))), "read: {read:?}");
}

#[tokio::test]
async fn callback_ignores_other_paths() {
    let cb = CallbackServer::bind().await.expect("bind");
    let uri = cb.redirect_uri();
    let base = uri.trim_end_matches("/cb").to_owned();
    let wait = tokio::spawn(cb.wait("s", Duration::from_secs(5)));

    let other = client()
        .get(format!("{base}/x?code=abc&state=s"))
        .send()
        .await
        .expect("answer");
    assert_eq!(other.status(), 404);
    client()
        .get(format!("{uri}?code=abc&state=s"))
        .send()
        .await
        .expect("callback answers");

    assert_eq!(
        wait.await.expect("wait task"),
        CallbackResult::Code("abc".to_owned())
    );
}

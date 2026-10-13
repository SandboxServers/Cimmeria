//! The uploader loop against a real local HTTP server, and the batch
//! encoding.

use super::*;
use crate::queue::channel;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

/// `encode_batch` produces a gzipped stream that decodes to
/// one JSON object per event, in the order pushed. Pins the
/// wire shape against the server's gzip + NDJSON parser.
#[test]
fn encode_batch_round_trip() {
    let events = vec![
        ClientNativeEvent {
            ts_ms: 100,
            seq: 0,
            target: "client.a".into(),
            level: "info".into(),
            fields: Default::default(),
        },
        ClientNativeEvent {
            ts_ms: 200,
            seq: 1,
            target: "client.b".into(),
            level: "debug".into(),
            fields: Default::default(),
        },
    ];
    let gz = encode_batch(&events).unwrap();
    let mut dec = flate2::read::GzDecoder::new(&gz[..]);
    let mut decoded = String::new();
    dec.read_to_string(&mut decoded).unwrap();
    let lines: Vec<&str> = decoded.lines().collect();
    assert_eq!(lines.len(), 2);
    // First line corresponds to event 0; type tag must be
    // `client_native` so the server's `enum TelemetryEvent`
    // dispatches to the right variant.
    assert!(lines[0].contains(r#""type":"client_native""#));
    assert!(lines[0].contains(r#""target":"client.a""#));
    assert!(lines[1].contains(r#""target":"client.b""#));
}

/// Value of the first request header named `name`, or `""` if absent.
fn header_value(headers: &[tiny_http::Header], name: &'static str) -> String {
    headers
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default()
}

/// **Uploader end-to-end with a real local HTTP server.**
/// Spawns `tiny_http` on a 127.0.0.1 port, runs the uploader
/// against it, pushes 3 events, asserts the server saw all 3
/// in one POST with the right gzip body + bearer header.
#[test]
fn uploader_ships_batch_to_local_server() {
    // Bind to an ephemeral port and let the OS pick.
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = server.server_addr().to_ip().unwrap();
    let url = format!(
        "http://{}:{}/api/telemetry/upload-chunk",
        addr.ip(),
        addr.port()
    );

    let received: Arc<std::sync::Mutex<Vec<(String, String, String)>>> =
        Arc::new(Default::default());
    let received_clone = received.clone();
    let server_handle = thread::spawn(move || {
        // Handle one request then drop the server.
        if let Ok(mut req) = server.recv() {
            let auth = header_value(req.headers(), "Authorization");
            let pid = header_value(req.headers(), "X-Cimmeria-Client-Pid");
            let mut body = Vec::new();
            req.as_reader().read_to_end(&mut body).unwrap();
            // Decode gzip → NDJSON
            let mut dec = flate2::read::GzDecoder::new(&body[..]);
            let mut decoded = String::new();
            dec.read_to_string(&mut decoded).unwrap();
            received_clone.lock().unwrap().push((auth, pid, decoded));
            req.respond(tiny_http::Response::from_string("ok")).ok();
        }
    });

    let (p, c) = channel();
    for i in 0..3 {
        p.try_emit(
            ClientNativeEvent::builder("client.test", "info").field("i", serde_json::json!(i)),
        );
    }
    // Drop the producer so the uploader's recv eventually
    // returns Disconnected and the loop exits.
    drop(p);

    let cfg = UploaderConfig {
        upload_endpoint: url,
        token: "test-token-XYZ".into(),
        expires_at_ms: 0,
        session_file: None,
        flush_interval: Duration::from_millis(100),
        max_batch: 10,
    };
    let exit = run_uploader(c, cfg, || false);
    server_handle.join().unwrap();

    assert_eq!(exit, UploaderExit::Disconnected);
    let got = received.lock().unwrap();
    assert_eq!(got.len(), 1, "exactly one POST with all 3 events batched");
    let (auth, pid, body) = &got[0];
    assert_eq!(auth, "Bearer test-token-XYZ");
    // The header is this test process's id, so a revert of the
    // header (or a wrong value) fails here.
    assert_eq!(pid, &std::process::id().to_string());
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 3);
    for (i, line) in lines.iter().enumerate() {
        assert!(
            line.contains(&format!(r#""i":{}"#, i)),
            "line {i} missing i={i}: {line}"
        );
    }
}

/// **Stop-callback drains and exits.** The DLL doesn't use
/// this in production (parks until process exit), but the test
/// path needs a way to tell the loop to stop without dropping
/// the producer side.
#[test]
fn stop_callback_exits_cleanly() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = server.server_addr().to_ip().unwrap();
    let url = format!("http://{}:{}/x", addr.ip(), addr.port());
    let server_done = Arc::new(AtomicBool::new(false));
    let server_done2 = server_done.clone();
    let server_handle = thread::spawn(move || {
        // Drain whatever requests come; mark done when we exit.
        while !server_done2.load(Ordering::Relaxed) {
            if let Ok(Some(req)) = server.recv_timeout(Duration::from_millis(100)) {
                req.respond(tiny_http::Response::from_string("ok")).ok();
            }
        }
    });

    let (p, c) = channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let upload = thread::spawn(move || {
        let cfg = UploaderConfig {
            upload_endpoint: url,
            token: "t".into(),
            expires_at_ms: 0,
            session_file: None,
            flush_interval: Duration::from_millis(50),
            max_batch: 100,
        };
        run_uploader(c, cfg, move || stop2.load(Ordering::Relaxed))
    });
    p.try_emit(ClientNativeEvent::builder("client.t", "info"));
    thread::sleep(Duration::from_millis(150));
    stop.store(true, Ordering::Relaxed);
    let exit = upload.join().unwrap();
    server_done.store(true, Ordering::Relaxed);
    server_handle.join().unwrap();
    assert_eq!(exit, UploaderExit::Stopped);
}

/// **Retains batch across a failed POST.** Server returns 503
/// once, then 200. The uploader must retry the same events on
/// the next cadence rather than dropping them after the 503.
///
/// Regression guard for the bug where `batch.clear()` ran
/// unconditionally inside the `if should_flush` arm — that
/// silently dropped any telemetry whose POST failed. Reverting
/// the fix flips this test to "0 events received" on the
/// retry POST.
///
/// TG-PIPE-13: the failed POST is counted. The channel is governed so the
/// stop path's final health row reports `upload_failures_total = 1` and
/// the last answer (`last_upload_status = 200`, the retry); before the
/// packet the failure was discarded and the row had no such fields.
#[test]
fn retains_batch_after_failed_post() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = server.server_addr().to_ip().unwrap();
    let url = format!("http://{}:{}/x", addr.ip(), addr.port());

    // Server returns 503 for the first request, 200 thereafter.
    // Capture the body of every request so the assertion can
    // verify the retry carries the original events.
    let bodies: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(Default::default());
    let bodies2 = bodies.clone();
    let server_handle = thread::spawn(move || {
        let mut request_n = 0;
        while let Ok(Some(mut req)) = server.recv_timeout(Duration::from_secs(2)) {
            let mut buf = Vec::new();
            req.as_reader().read_to_end(&mut buf).unwrap();
            let mut dec = flate2::read::GzDecoder::new(&buf[..]);
            let mut decoded = String::new();
            dec.read_to_string(&mut decoded).unwrap();
            bodies2.lock().unwrap().push(decoded);
            let resp = if request_n == 0 {
                tiny_http::Response::from_string("oops")
                    .with_status_code(503)
                    .boxed()
            } else {
                tiny_http::Response::from_string("ok").boxed()
            };
            req.respond(resp).ok();
            request_n += 1;
        }
    });

    let (p, c) = crate::queue::governed_channel(crate::governor::GovernorConfig::default());
    // 2 events; tight flush interval so we get multiple cadences.
    p.try_emit(ClientNativeEvent::builder("client.retry", "info").field("i", serde_json::json!(0)));
    p.try_emit(ClientNativeEvent::builder("client.retry", "info").field("i", serde_json::json!(1)));

    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let upload = thread::spawn(move || {
        let cfg = UploaderConfig {
            upload_endpoint: url,
            token: "t".into(),
            expires_at_ms: 0,
            session_file: None,
            flush_interval: Duration::from_millis(50),
            max_batch: 100,
        };
        run_uploader(c, cfg, move || stop2.load(Ordering::Relaxed))
    });

    // Give the uploader enough time to attempt the 503 POST and
    // then the 200 retry on the next cadence.
    thread::sleep(Duration::from_millis(400));
    stop.store(true, Ordering::Relaxed);
    // The producer outlives the uploader, as in the DLL: dropping it first
    // can end the loop through `Disconnected`, which skips the stop path
    // and its final health row.
    assert_eq!(upload.join().unwrap(), UploaderExit::Stopped);
    drop(p);
    server_handle.join().unwrap();

    let got = bodies.lock().unwrap();
    assert!(
        got.len() >= 2,
        "expected at least 2 POSTs (503 + retry), got {}",
        got.len()
    );
    // First POST was 503 — but should still have carried both
    // events. Second POST is the retry — must also carry both
    // events (we retained the batch).
    for (idx, body) in got.iter().enumerate().take(2) {
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "POST #{idx} expected 2 events, got {}: {body}",
            lines.len()
        );
        assert!(lines[0].contains(r#""i":0"#), "POST #{idx} missing i=0");
        assert!(lines[1].contains(r#""i":1"#), "POST #{idx} missing i=1");
    }
    // The stop path's last POST holds the final health row, which counts
    // the one 503.
    let last = got.last().unwrap();
    let health: serde_json::Value = last
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .find(|e| e["target"] == "client.telemetry.health")
        .unwrap_or_else(|| panic!("no health row in the last POST: {last}"));
    assert_eq!(health["fields"]["final"], serde_json::json!(true));
    assert_eq!(
        health["fields"]["upload_failures_total"],
        serde_json::json!(1)
    );
    assert_eq!(
        health["fields"]["last_upload_status"],
        serde_json::json!(200)
    );
    assert_eq!(
        health["level"], "warn",
        "a failed POST since the last report"
    );
}

/// **Batch caps at max_batch.** Stuff the queue with more
/// than `max_batch` events; the uploader ships them in
/// multiple POSTs of size ≤ max_batch.
#[test]
fn batch_caps_at_max_batch() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = server.server_addr().to_ip().unwrap();
    let url = format!("http://{}:{}/x", addr.ip(), addr.port());
    let request_count = Arc::new(AtomicUsize::new(0));
    let request_count2 = request_count.clone();
    let server_handle = thread::spawn(move || {
        while let Ok(Some(mut req)) = server.recv_timeout(Duration::from_secs(2)) {
            let mut buf = Vec::new();
            req.as_reader().read_to_end(&mut buf).unwrap();
            request_count2.fetch_add(1, Ordering::Relaxed);
            req.respond(tiny_http::Response::from_string("ok")).ok();
        }
    });

    let (p, c) = channel();
    // 25 events with max_batch=10 → at least 3 POSTs (10 + 10 + 5).
    for i in 0..25 {
        p.try_emit(ClientNativeEvent::builder("t", "info").field("i", serde_json::json!(i)));
    }
    drop(p);
    let cfg = UploaderConfig {
        upload_endpoint: url,
        token: "t".into(),
        expires_at_ms: 0,
        session_file: None,
        flush_interval: Duration::from_millis(50),
        max_batch: 10,
    };
    run_uploader(c, cfg, || false);
    server_handle.join().unwrap();
    let count = request_count.load(Ordering::Relaxed);
    assert!(count >= 3, "expected at least 3 batched POSTs, got {count}");
}

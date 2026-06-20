//! In-DLL uploader thread.
//!
//! Owns the queue's `Consumer` end. Wakes every
//! `flush_interval_ms` (or sooner if `recv_timeout` returns an
//! event), drains a batch up to `max_batch`, gzips the NDJSON
//! payload, POSTs to the launcher's HMAC-protected
//! `/api/telemetry/upload-chunk` endpoint with the per-session
//! Bearer token.
//!
//! # Thread shape
//!
//! Plain `std::thread::spawn` — no tokio. `ureq` is a single-
//! threaded sync HTTP client; one of these per process is plenty.
//! The thread parks for the lifetime of the DLL (which is the
//! lifetime of SGW.exe). Phase 1's `bootstrap_main` previously
//! returned immediately and let the bootstrap thread exit; now it
//! transfers control to [`run_uploader`] which never returns
//! under normal operation.
//!
//! # Failure modes (current implementation)
//!
//! The Phase-2 uploader is deliberately minimal — it ships batches
//! and retains them on failure but does not yet parse server hints
//! or refresh credentials. Phase 3 will add the more sophisticated
//! handling described in [docs/architecture/dev-session-telemetry.md].
//!
//! - **401 from server** — token rejected. Treated like any other
//!   non-2xx: the batch is retained for retry on the next cadence
//!   and the failure is silently discarded by the caller. Phase 3
//!   will re-read `current-session.json` to pick up a refreshed
//!   token (the launcher rotates at 75% TTL).
//! - **5xx from server** — overload / outage. Same path as 401:
//!   retained for retry on the next cadence. `Retry-After` is not
//!   parsed today; Phase 3 will honor it.
//! - **Network error** — same path. The batch (which has already
//!   been popped from the queue) is retained in the uploader's
//!   `Vec` and re-attempted on the next flush cadence. When the
//!   retained batch hits `max_batch` capacity, the oldest half is
//!   dropped to bound memory at ~2× `max_batch` events.
//! - **Disconnected channel** — Producer side hung up. Treated as
//!   "uploader done" and the thread exits.
//!
//! Drops also happen at the queue itself when the producer side
//! overflows the ring (see [`crate::queue`]) — those are accounted
//! for separately and never reach the uploader.

use std::io::Write;
use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::events::{ClientNativeEvent, TelemetryEvent};
use crate::queue::Consumer;

/// Uploader runtime configuration. Built from
/// `current-session.json`'s `telemetry` block by the bootstrap
/// thread; passed to [`run_uploader`].
#[derive(Debug, Clone)]
pub struct UploaderConfig {
    /// Full endpoint URL incl. scheme + host + path
    /// (`https://.../api/telemetry/upload-chunk`). Mirrors the
    /// launcher's `TelemetryBlock.upload_endpoint`.
    pub upload_endpoint: String,
    /// Bearer HMAC token minted by `/api/auth/dev-session`. Used
    /// verbatim in the `Authorization: Bearer ...` header.
    pub token: String,
    /// Flush cadence — how often the uploader wakes to ship a
    /// batch even when the queue isn't full. Launcher default is
    /// 2000 ms; the DLL inherits whatever the launcher wrote.
    pub flush_interval: Duration,
    /// Hard cap on events per POST. Bounds the worst-case body
    /// size when the queue fills between flushes. 1000 events ×
    /// ~256 bytes/event ≈ 256 KiB pre-gzip, well under the 16 MiB
    /// server-side body limit.
    pub max_batch: usize,
}

impl UploaderConfig {
    /// Default cadence + batch when the session-loaded values are
    /// 0 (interpreted as "use defaults"). The launcher's writer
    /// always populates these, but the DLL is defensive against a
    /// partially-written file.
    pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_millis(2000);
    pub const DEFAULT_MAX_BATCH: usize = 1000;

    /// Build from a `session::TelemetryBlock`, applying defaults
    /// for any zero-valued field.
    pub fn from_session(block: &crate::session::TelemetryBlock) -> Self {
        Self {
            upload_endpoint: block.upload_endpoint.clone(),
            token: block.token.clone(),
            flush_interval: if block.flush_interval_ms == 0 {
                Self::DEFAULT_FLUSH_INTERVAL
            } else {
                Duration::from_millis(block.flush_interval_ms)
            },
            max_batch: Self::DEFAULT_MAX_BATCH,
        }
    }
}

/// Outcome reported back when the uploader thread finally exits.
/// Lets tests assert what made it stop (vs hanging on a panic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploaderExit {
    /// Producer side hung up — Consumer's recv returned
    /// `Disconnected`. Normal shutdown for tests; in the DLL this
    /// only happens if the process is tearing down.
    Disconnected,
    /// The thread was asked to stop via the `should_stop`
    /// callback. Only used by tests.
    Stopped,
}

/// Run the uploader loop until the queue is disconnected or
/// `should_stop()` returns true. In production the DLL passes a
/// `|| false` callback and the loop runs for the lifetime of
/// SGW.exe.
///
/// The blocking shape is: `recv_timeout(flush_interval)` to wait
/// for the next event or wake on cadence; then drain up to
/// `max_batch` with `try_recv` (non-blocking); then ship if the
/// batch is non-empty.
pub fn run_uploader(
    consumer: Consumer,
    cfg: UploaderConfig,
    mut should_stop: impl FnMut() -> bool,
) -> UploaderExit {
    let client = build_agent();
    let mut batch: Vec<ClientNativeEvent> = Vec::with_capacity(cfg.max_batch);
    let mut last_flush = Instant::now();

    loop {
        if should_stop() {
            // Drain anything still in the queue + ship one last
            // batch on the way out.
            drain_into(&consumer, &mut batch, cfg.max_batch);
            if !batch.is_empty() {
                let _ = post_batch(&client, &cfg, &batch);
            }
            return UploaderExit::Stopped;
        }

        // Wait for the next event. Pick the timeout so we wake on
        // cadence:
        //  - empty batch: wait the full flush_interval (nothing to
        //    flush, no reason to wake sooner). This avoids a 1 ms
        //    busy-spin when no events arrive but `last_flush` is
        //    older than `flush_interval`.
        //  - non-empty batch: wait at most `flush_interval -
        //    last_flush.elapsed()`, floored at 1 ms, so we ship
        //    pending events on cadence even if the queue went quiet.
        let recv_timeout = if batch.is_empty() {
            cfg.flush_interval
        } else {
            cfg.flush_interval
                .saturating_sub(last_flush.elapsed())
                .max(Duration::from_millis(1))
        };
        match recv_timeout_raw(&consumer, recv_timeout) {
            Ok(ev) => batch.push(ev),
            Err(RecvTimeoutError::Timeout) => { /* fall through to flush */ }
            Err(RecvTimeoutError::Disconnected) => {
                if !batch.is_empty() {
                    let _ = post_batch(&client, &cfg, &batch);
                }
                return UploaderExit::Disconnected;
            }
        }

        // Drain anything else immediately available without
        // blocking. Caps at `max_batch - 1` (we already pushed at
        // most one above).
        drain_into(&consumer, &mut batch, cfg.max_batch);

        // Ship when either the batch hit a meaningful size OR the
        // flush interval elapsed since the last successful POST.
        let should_flush = !batch.is_empty()
            && (batch.len() >= cfg.max_batch || last_flush.elapsed() >= cfg.flush_interval);
        if should_flush {
            match post_batch(&client, &cfg, &batch) {
                Ok(()) => {
                    batch.clear();
                }
                Err(_) => {
                    // Network/HTTP failure — retain the batch for
                    // retry on the next cadence rather than dropping
                    // it. Bound memory: if the retained batch is
                    // already at `max_batch`, drop the oldest half
                    // to leave room for new events. This caps worst-
                    // case memory at ~2 × max_batch events while
                    // preferring newer telemetry over the very
                    // oldest.
                    if batch.len() >= cfg.max_batch {
                        batch.drain(..batch.len() / 2);
                    }
                }
            }
            last_flush = Instant::now();
        }
    }
}

/// Build the ureq agent we use for every POST. Single instance per
/// thread — ureq keeps the underlying TCP connection alive across
/// calls, which matters for the rapid-flush case where we ship
/// every 2 s.
fn build_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .build()
}

/// Pop as many events as we can from `consumer` without blocking,
/// up to `max_batch` total in `batch`.
fn drain_into(consumer: &Consumer, batch: &mut Vec<ClientNativeEvent>, max_batch: usize) {
    while batch.len() < max_batch {
        match consumer.try_recv() {
            Some(ev) => batch.push(ev),
            None => break,
        }
    }
}

/// Serialize as NDJSON, gzip, POST with the Bearer token.
///
/// Returns `Err` on any network or non-2xx HTTP response. The
/// caller (the uploader loop) retains the batch in memory on
/// failure and re-attempts on the next flush cadence, so the
/// delivery shape is at-least-once *with possible duplicates* if
/// the server actually processed the batch before its response
/// failed (since we cannot distinguish that case from a true POST
/// failure). The server's `/api/telemetry/upload-chunk` handler is
/// designed to be idempotent on the `(install_id, session_id,
/// seq)` tuple to absorb those duplicates.
fn post_batch(
    client: &ureq::Agent,
    cfg: &UploaderConfig,
    batch: &[ClientNativeEvent],
) -> Result<(), PostError> {
    let body = encode_batch(batch)?;
    let resp = client
        .post(&cfg.upload_endpoint)
        .set("Authorization", &format!("Bearer {}", cfg.token))
        .set("Content-Type", "application/x-ndjson")
        .set("Content-Encoding", "gzip")
        .send_bytes(&body)
        .map_err(|e| PostError::Network(e.to_string()))?;
    let status = resp.status();
    if !(200..300).contains(&status) {
        return Err(PostError::Http {
            status,
            body: resp.into_string().unwrap_or_default(),
        });
    }
    Ok(())
}

/// NDJSON encode → gzip. Each event becomes one
/// `TelemetryEvent::ClientNative(...)` JSON object on its own line.
fn encode_batch(batch: &[ClientNativeEvent]) -> Result<Vec<u8>, PostError> {
    let mut gz = GzEncoder::new(
        Vec::with_capacity(batch.len() * 200),
        Compression::default(),
    );
    for ev in batch {
        let wrapped = TelemetryEvent::ClientNative(ev.clone());
        serde_json::to_writer(&mut gz, &wrapped).map_err(PostError::Serialize)?;
        gz.write_all(b"\n").map_err(PostError::Io)?;
    }
    gz.finish().map_err(PostError::Io)
}

#[derive(Debug, thiserror::Error)]
enum PostError {
    #[error("network: {0}")]
    Network(String),
    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("serialize: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
}

// Helper used by the uploader loop only — distinguishes Timeout
// from Disconnected, which the public `Consumer::recv_timeout`
// collapses to `None`. Uses the crate-internal `raw()` accessor.
fn recv_timeout_raw(
    consumer: &Consumer,
    timeout: Duration,
) -> Result<ClientNativeEvent, RecvTimeoutError> {
    consumer.raw().recv_timeout(timeout)
}

#[cfg(test)]
mod tests {
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

        let received: Arc<std::sync::Mutex<Vec<(String, String)>>> = Arc::new(Default::default());
        let received_clone = received.clone();
        let server_handle = thread::spawn(move || {
            // Handle one request then drop the server.
            if let Ok(mut req) = server.recv() {
                let auth = req
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Authorization"))
                    .map(|h| h.value.as_str().to_string())
                    .unwrap_or_default();
                let mut body = Vec::new();
                req.as_reader().read_to_end(&mut body).unwrap();
                // Decode gzip → NDJSON
                let mut dec = flate2::read::GzDecoder::new(&body[..]);
                let mut decoded = String::new();
                dec.read_to_string(&mut decoded).unwrap();
                received_clone.lock().unwrap().push((auth, decoded));
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
            flush_interval: Duration::from_millis(100),
            max_batch: 10,
        };
        let exit = run_uploader(c, cfg, || false);
        server_handle.join().unwrap();

        assert_eq!(exit, UploaderExit::Disconnected);
        let got = received.lock().unwrap();
        assert_eq!(got.len(), 1, "exactly one POST with all 3 events batched");
        let (auth, body) = &got[0];
        assert_eq!(auth, "Bearer test-token-XYZ");
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

        let (p, c) = channel();
        // 2 events; tight flush interval so we get multiple cadences.
        p.try_emit(
            ClientNativeEvent::builder("client.retry", "info").field("i", serde_json::json!(0)),
        );
        p.try_emit(
            ClientNativeEvent::builder("client.retry", "info").field("i", serde_json::json!(1)),
        );

        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let upload = thread::spawn(move || {
            let cfg = UploaderConfig {
                upload_endpoint: url,
                token: "t".into(),
                flush_interval: Duration::from_millis(50),
                max_batch: 100,
            };
            run_uploader(c, cfg, move || stop2.load(Ordering::Relaxed))
        });

        // Give the uploader enough time to attempt the 503 POST and
        // then the 200 retry on the next cadence.
        thread::sleep(Duration::from_millis(400));
        stop.store(true, Ordering::Relaxed);
        drop(p);
        upload.join().unwrap();
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
            flush_interval: Duration::from_millis(50),
            max_batch: 10,
        };
        run_uploader(c, cfg, || false);
        server_handle.join().unwrap();
        let count = request_count.load(Ordering::Relaxed);
        assert!(count >= 3, "expected at least 3 batched POSTs, got {count}");
    }
}

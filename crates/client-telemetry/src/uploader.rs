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
//!   and the failure is counted (`upload_failures_total` and
//!   `last_upload_status` on the health row, TG-PIPE-13). Phase 3
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
//! for separately and never reach the uploader. Both counts, the ring's
//! and the discarded-after-failed-POST count above, are reported by the
//! governor's `client.telemetry.health` event (see [`crate::governor`]),
//! next to the failed-POST count.

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
            upload_endpoint: chunk_url(&block.upload_endpoint),
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

/// The chunk URL for a session file's `upload_endpoint`.
///
/// The dev-session response, and so the launcher's `current-session.json`,
/// carries the upload *base* (`https://host/api/telemetry`): the launcher
/// appends `/upload-chunk` and `/upload-bundle` itself. The DLL used to
/// POST to the value verbatim, so every batch from a launcher-written
/// session went to the base path and was refused, and no DLL event ever
/// reached the server. Accept both shapes: a value that already names the
/// chunk route is used as is.
pub fn chunk_url(endpoint: &str) -> String {
    let base = endpoint.trim().trim_end_matches('/');
    if base.ends_with("/upload-chunk") {
        base.to_string()
    } else {
        format!("{base}/upload-chunk")
    }
}

/// How often the idle loop wakes to look at `should_stop`. Well under
/// [`crate::shutdown::FINAL_FLUSH_BUDGET`], so a stop request starts the
/// final flush within a tick instead of after a whole flush interval.
const STOP_POLL: Duration = Duration::from_millis(100);

/// Whole-request limit (connect, send and read) for the final POST. Shorter
/// than [`crate::shutdown::FINAL_FLUSH_BUDGET`] so the flush normally
/// finishes inside the wait; the agent's 10 s connect and 30 s read limits
/// are for the ordinary cadence.
pub const FINAL_POST_TIMEOUT: Duration = Duration::from_millis(1200);

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
/// `should_stop()` returns true. In production the DLL passes
/// `shutdown::global().stop_requested()`, which the exit hooks raise
/// (TG-PIPE-03); until then the loop runs for the lifetime of SGW.exe.
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
    // Events discarded after failed POSTs. Reported by the governor's
    // health event, so a long outage is visible once the link returns.
    let mut upload_dropped: u64 = 0;

    loop {
        if should_stop() {
            // Drain anything still in the queue + ship one last batch on
            // the way out. `governor_finish` appends the closing rollups and
            // the health row marked `final: true`. The POST is capped at
            // `FINAL_POST_TIMEOUT` so an exiting process is never held by a
            // slow server (see `crate::shutdown`).
            drain_into(&consumer, &mut batch, cfg.max_batch);
            consumer.governor_finish(upload_dropped, &mut batch);
            if !batch.is_empty() {
                let result = post_batch_within(&client, &cfg, &batch, Some(FINAL_POST_TIMEOUT));
                // No health row follows this one, so the local log is the
                // only place a failed final POST can show.
                if let Err(e) = record(&consumer, result) {
                    crate::log::line(format_args!(
                        "final POST of {} events failed: {e}",
                        batch.len()
                    ));
                }
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
        }
        // Wake often enough to notice `should_stop` inside the exit budget.
        .min(STOP_POLL);
        match recv_timeout_raw(&consumer, recv_timeout) {
            Ok(ev) => batch.push(ev),
            Err(RecvTimeoutError::Timeout) => { /* fall through to flush */ }
            Err(RecvTimeoutError::Disconnected) => {
                if !batch.is_empty() {
                    let _ = record(&consumer, post_batch(&client, &cfg, &batch));
                }
                return UploaderExit::Disconnected;
            }
        }

        // Drain anything else immediately available without
        // blocking. Caps at `max_batch - 1` (we already pushed at
        // most one above).
        drain_into(&consumer, &mut batch, cfg.max_batch);
        // Rollups, repeat events and health, on the governor's own cadence.
        consumer.governor_tick(upload_dropped, &mut batch);

        // Ship when either the batch hit a meaningful size OR the
        // flush interval elapsed since the last successful POST.
        let should_flush = !batch.is_empty()
            && (batch.len() >= cfg.max_batch || last_flush.elapsed() >= cfg.flush_interval);
        if should_flush {
            match record(&consumer, post_batch(&client, &cfg, &batch)) {
                Ok(()) => {
                    batch.clear();
                }
                Err(_) => {
                    // Network/HTTP failure, counted by `record` for the
                    // health row's `upload_failures_total` and
                    // `last_upload_status` (TG-PIPE-13). Retain the batch for
                    // retry on the next cadence rather than dropping
                    // it. Bound memory: if the retained batch is
                    // already at `max_batch`, drop the oldest half
                    // to leave room for new events. This caps worst-
                    // case memory at ~2 × max_batch events while
                    // preferring newer telemetry over the very
                    // oldest.
                    if batch.len() >= cfg.max_batch {
                        let n = batch.len() / 2;
                        batch.drain(..n);
                        upload_dropped += n as u64;
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

/// Header carrying this process's id on every chunk. One lab session
/// spans several launches, so the server needs the pid to tell them apart.
const CLIENT_PID_HEADER: &str = "X-Cimmeria-Client-Pid";

/// Serialize as NDJSON, gzip, POST with the Bearer token. Returns the HTTP
/// status on a 2xx answer.
///
/// Returns `Err` on any network or non-2xx HTTP response. The
/// caller (the uploader loop) retains the batch in memory on
/// failure and re-attempts on the next flush cadence, so the
/// delivery shape is at-least-once *with possible duplicates* if
/// the server actually processed the batch before its response
/// failed (since we cannot distinguish that case from a true POST
/// failure). The server does **not** absorb those duplicates:
/// `/api/telemetry/upload-chunk` replays every row it parses and keeps
/// no `(session_id, seq)` set (see `admin-api` `routes/telemetry/mod.rs`),
/// so a retried batch the server had already taken shows up twice in
/// SigNoz, with the same `seq`.
fn post_batch(
    client: &ureq::Agent,
    cfg: &UploaderConfig,
    batch: &[ClientNativeEvent],
) -> Result<u16, PostError> {
    post_batch_within(client, cfg, batch, None)
}

/// Count a POST's outcome on the consumer (TG-PIPE-13): a failure bumps
/// `upload_failures_total`, and every answer sets `last_upload_status`.
fn record(consumer: &Consumer, result: Result<u16, PostError>) -> Result<(), PostError> {
    match result {
        Ok(status) => {
            consumer.record_upload_ok(status);
            Ok(())
        }
        Err(e) => {
            consumer.record_upload_failure(e.status());
            Err(e)
        }
    }
}

/// [`post_batch`] with an optional whole-request time limit (the final
/// flush at exit passes one; the ordinary cadence does not).
fn post_batch_within(
    client: &ureq::Agent,
    cfg: &UploaderConfig,
    batch: &[ClientNativeEvent],
    limit: Option<Duration>,
) -> Result<u16, PostError> {
    let body = encode_batch(batch)?;
    let mut req = client
        .post(&cfg.upload_endpoint)
        .set("Authorization", &format!("Bearer {}", cfg.token))
        .set("Content-Type", "application/x-ndjson")
        .set("Content-Encoding", "gzip")
        .set(CLIENT_PID_HEADER, &std::process::id().to_string());
    if let Some(limit) = limit {
        req = req.timeout(limit);
    }
    // ureq 2 reports a 4xx/5xx answer as `Error::Status`, not as a
    // response; keep its status so the health row can say what the
    // server answered.
    let resp = match req.send_bytes(&body) {
        Ok(resp) => resp,
        Err(ureq::Error::Status(status, resp)) => {
            return Err(PostError::Http {
                status,
                body: resp.into_string().unwrap_or_default(),
            })
        }
        Err(e) => return Err(PostError::Network(e.to_string())),
    };
    let status = resp.status();
    if !(200..300).contains(&status) {
        return Err(PostError::Http {
            status,
            body: resp.into_string().unwrap_or_default(),
        });
    }
    Ok(status)
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

impl PostError {
    /// The server's HTTP status, when it answered at all.
    fn status(&self) -> Option<u16> {
        match self {
            PostError::Http { status, .. } => Some(*status),
            _ => None,
        }
    }
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
#[path = "uploader_endpoint_tests.rs"]
mod endpoint_tests;

#[cfg(test)]
#[path = "uploader_final_flush_tests.rs"]
mod final_flush_tests;

#[cfg(test)]
#[path = "uploader_tests.rs"]
mod tests;

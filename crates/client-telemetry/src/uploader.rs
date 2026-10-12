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
//! # Failure modes
//!
//! Every failed POST keeps its batch for a later attempt and is counted
//! (`upload_failures_total` and `last_upload_status` on the health row,
//! TG-PIPE-13). *When* the next attempt goes, and with which token, is
//! [`crate::upload_gate::UploadGate`]'s decision:
//!
//! - **401 or 403 from server** — token refused (`token_expired` and the
//!   like). The uploader re-reads `current-session.json`. A different token
//!   there is swapped in and the batch is retried at once (no launcher
//!   rewrites the file on refresh yet; see [`crate::upload_gate`]). The
//!   same token, or no readable
//!   file, pauses uploads: holds that start at the flush interval and double
//!   up to 10 min ([`crate::upload_gate::BACKOFF_CAP`]), with a re-read of
//!   the file at each deadline so a later rotation recovers. Before
//!   2026-10-12 a 401 was retried every flush, forever: one lab client
//!   whose token expired sent about three refused POSTs every 12 s for
//!   4.5 h.
//! - **An empty token** — never sent, the final POST at exit included; the
//!   uploader holds and re-reads on the same backoff.
//! - **A token past the file's `expires_at_ms` by this machine's clock** —
//!   held the same way, but sent as one probe at once and then once per
//!   10 min, because this clock may be ahead of the server's. A probe the
//!   server accepts restores normal uploads; the final POST at exit never
//!   spends a probe.
//! - **5xx, another 4xx, or a network error** — the same bounded backoff.
//!   A `Retry-After` in whole seconds that asks for longer wins (capped
//!   too); the HTTP-date form is ignored.
//! - **Memory** — the batch (already popped from the queue) stays in the
//!   uploader's `Vec`. When a failed POST leaves it at `max_batch`, the
//!   oldest half is dropped. While a hold runs, events from the ring stop
//!   at `max_batch` (the ring absorbs, and counts, the overflow), but the
//!   governor's rollup and health rows and the pause report still land in
//!   the batch, so every loop pass also drops the oldest half once it
//!   reaches 2× `max_batch`. Either way memory stays under ~2× `max_batch`
//!   events, and every dropped event is counted in the health row's
//!   `upload_dropped_total`.
//! - **Disconnected channel** — Producer side hung up. Treated as
//!   "uploader done" and the thread exits.
//!
//! An auth pause is reported once when it starts
//! ([`UPLOAD_PAUSED_TARGET`]) and once when a POST goes through again
//! ([`UPLOAD_RESUMED_TARGET`]), each as one event in the batch and one line
//! in the local log. The paused row reaches the server with the first
//! accepted POST, next to the resumed one; a client that never recovers has
//! only the log line.
//!
//! Drops also happen at the queue itself when the producer side
//! overflows the ring (see [`crate::queue`]) — those are accounted
//! for separately and never reach the uploader. Both counts, the ring's
//! and the discarded-after-failed-POST count above, are reported by the
//! governor's `client.telemetry.health` event (see [`crate::governor`]),
//! next to the failed-POST count.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::RecvTimeoutError;

use crate::events::ClientNativeEvent;
use crate::queue::Consumer;
use crate::upload_gate::{AfterRefusal, Credentials, Transition, UploadGate, BACKOFF_CAP};

#[path = "uploader_post.rs"]
mod post;
#[cfg(test)]
use post::encode_batch;
use post::{build_agent, post_batch_within, record, PostError};

/// Target of the one event written when uploads pause for want of a usable
/// token.
pub const UPLOAD_PAUSED_TARGET: &str = "client.telemetry.upload_paused";

/// Target of the one event written when uploads go through again.
pub const UPLOAD_RESUMED_TARGET: &str = "client.telemetry.upload_resumed";

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
    /// verbatim in the `Authorization: Bearer ...` header, until a re-read
    /// of [`UploaderConfig::session_file`] swaps in a newer one.
    pub token: String,
    /// The token's expiry (ms since the epoch); `0` = not stated. A token
    /// past it is never sent.
    pub expires_at_ms: i64,
    /// The `current-session.json` the session came from, re-read after an
    /// auth refusal and while a pause lasts. `None` (tests) never re-reads.
    pub session_file: Option<PathBuf>,
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
    /// for any zero-valued field. [`UploaderConfig::session_file`] starts
    /// unset; the bootstrap thread fills it in.
    pub fn from_session(block: &crate::session::TelemetryBlock) -> Self {
        Self {
            upload_endpoint: chunk_url(&block.upload_endpoint),
            token: block.token.clone(),
            expires_at_ms: block.expires_at_ms,
            session_file: None,
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
/// batch is non-empty and the [`UploadGate`] lets a POST go.
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
    let mut gate = UploadGate::new(
        Credentials {
            token: cfg.token.clone(),
            expires_at_ms: cfg.expires_at_ms,
        },
        cfg.flush_interval,
        BACKOFF_CAP,
    );
    let reload = || cfg.session_file.as_deref().and_then(reload_credentials);

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
                if gate.ready_for_final(wall_ms(), reload) {
                    let result = post_batch_within(
                        &client,
                        &cfg.upload_endpoint,
                        gate.token(),
                        &batch,
                        Some(FINAL_POST_TIMEOUT),
                    );
                    // No health row follows this one, so the local log is the
                    // only place a failed final POST can show.
                    if let Err(e) = record(&consumer, result) {
                        crate::log::line(format_args!(
                            "final POST of {} events failed: {e}",
                            batch.len()
                        ));
                    }
                } else {
                    crate::log::line(format_args!(
                        "final POST of {} events skipped: no usable token",
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
        //  - non-empty batch: wait until the next POST could go: the
        //    flush cadence (`flush_interval - last_flush.elapsed()`), or
        //    the end of the gate's hold when that is later, floored at
        //    1 ms. Without the hold term a held batch whose cadence had
        //    passed spun at 1 ms for the whole hold (or for good with a
        //    token that looks expired), ticking the governor's mutex.
        let recv_timeout = if batch.is_empty() {
            cfg.flush_interval
        } else {
            let now = Instant::now();
            let cadence = cfg.flush_interval.saturating_sub(last_flush.elapsed());
            let hold = gate
                .next_attempt()
                .map_or(Duration::ZERO, |t| t.saturating_duration_since(now));
            cadence.max(hold).max(Duration::from_millis(1))
        }
        // Wake often enough to notice `should_stop` inside the exit budget.
        .min(STOP_POLL);
        match recv_timeout_raw(&consumer, recv_timeout) {
            Ok(ev) => batch.push(ev),
            Err(RecvTimeoutError::Timeout) => { /* fall through to flush */ }
            Err(RecvTimeoutError::Disconnected) => {
                if !batch.is_empty() && gate.ready_for_final(wall_ms(), reload) {
                    let _ = record(
                        &consumer,
                        post_batch_within(
                            &client,
                            &cfg.upload_endpoint,
                            gate.token(),
                            &batch,
                            None,
                        ),
                    );
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
        // flush interval elapsed since the last POST, and the gate is not
        // holding uploads off (backoff, or no usable token).
        let should_flush = !batch.is_empty()
            && (batch.len() >= cfg.max_batch || last_flush.elapsed() >= cfg.flush_interval)
            && gate.ready(Instant::now(), wall_ms(), reload);
        if should_flush {
            match ship(&client, &consumer, &cfg, &mut gate, &batch, reload) {
                Ok(()) => {
                    batch.clear();
                }
                Err(_) => {
                    // Network/HTTP failure, counted by `record` for the
                    // health row's `upload_failures_total` and
                    // `last_upload_status` (TG-PIPE-13). Retain the batch for
                    // a later attempt (when, is the gate's call) rather than
                    // dropping it. Bound memory: if the retained batch is
                    // already at `max_batch`, drop the oldest half
                    // to leave room for new events. This caps worst-
                    // case memory at ~2 × max_batch events while
                    // preferring newer telemetry over the very
                    // oldest.
                    if batch.len() >= cfg.max_batch {
                        upload_dropped += trim_oldest_half(&mut batch);
                    }
                }
            }
            last_flush = Instant::now();
        }
        // After a cleared batch, so a resume row ships with the next POST
        // instead of being cleared with the batch it followed.
        report_transitions(&consumer, &mut gate, &mut batch);
        // While a hold runs nothing ships, but the governor keeps appending
        // rollup and health rows past `max_batch`: bound the batch here
        // too, not only after a failed POST.
        if batch.len() >= 2 * cfg.max_batch {
            upload_dropped += trim_oldest_half(&mut batch);
        }
    }
}

/// Drop the oldest half of `batch`, preferring newer telemetry; returns how
/// many events went.
fn trim_oldest_half(batch: &mut Vec<ClientNativeEvent>) -> u64 {
    let n = batch.len() / 2;
    batch.drain(..n);
    n as u64
}

/// One POST of `batch` through the gate: an auth refusal re-reads the
/// session file and retries once at once when it carries a new token; every
/// other outcome is handed to the gate to schedule the next attempt.
fn ship(
    client: &ureq::Agent,
    consumer: &Consumer,
    cfg: &UploaderConfig,
    gate: &mut UploadGate,
    batch: &[ClientNativeEvent],
    reload: impl Fn() -> Option<Credentials>,
) -> Result<(), PostError> {
    let mut retried = false;
    loop {
        let result = post_batch_within(client, &cfg.upload_endpoint, gate.token(), batch, None);
        let now = Instant::now();
        match record(consumer, result) {
            Ok(()) => {
                gate.on_success(now);
                return Ok(());
            }
            Err(e) => match e.auth_status() {
                Some(status) => match gate.on_auth_refused(now, wall_ms(), status, &reload) {
                    AfterRefusal::RetryNow if !retried => retried = true,
                    _ => return Err(e),
                },
                None => {
                    gate.on_transient(now, e.retry_after());
                    return Err(e);
                }
            },
        }
    }
}

/// Turn the gate's pause and resume transitions into one event each in
/// `batch` and one line each in the local log. The local line matters for
/// a pause that never ends: its event never reaches the server.
fn report_transitions(
    consumer: &Consumer,
    gate: &mut UploadGate,
    batch: &mut Vec<ClientNativeEvent>,
) {
    for t in gate.take_transitions() {
        let ev = match t {
            Transition::Paused {
                reason,
                status,
                retry_in,
            } => {
                crate::log::line(format_args!(
                    "uploads paused: token {} (status {status:?}); re-reading the session file with backoff, first retry in {} ms",
                    reason.as_str(),
                    retry_in.as_millis()
                ));
                let mut b = ClientNativeEvent::builder(UPLOAD_PAUSED_TARGET, "warn")
                    .field("reason", reason.as_str())
                    .field(
                        "retry_in_ms",
                        serde_json::json!(retry_in.as_millis() as u64),
                    );
                if let Some(s) = status {
                    b = b.field("status", serde_json::json!(s));
                }
                b
            }
            Transition::Resumed {
                paused_for,
                attempts,
            } => {
                crate::log::line(format_args!(
                    "uploads resumed after {} ms and {attempts} held attempts",
                    paused_for.as_millis()
                ));
                ClientNativeEvent::builder(UPLOAD_RESUMED_TARGET, "info")
                    .field(
                        "paused_ms",
                        serde_json::json!(paused_for.as_millis() as u64),
                    )
                    .field("attempts", serde_json::json!(attempts))
            }
        };
        batch.push(consumer.stamp(ev));
    }
}

/// The token half of the session file at `path`, or `None` when it is gone,
/// unreadable, unparsable or switched off (any of which means "no new
/// token"; the gate keeps backing off).
fn reload_credentials(path: &Path) -> Option<Credentials> {
    crate::session::load_session(path)
        .ok()
        .map(|s| Credentials {
            token: s.telemetry.token,
            expires_at_ms: s.telemetry.expires_at_ms,
        })
}

/// Wall-clock ms since the epoch, for comparing against `expires_at_ms`.
fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
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
#[path = "uploader_auth_tests.rs"]
mod auth_tests;

#[cfg(test)]
#[path = "uploader_endpoint_tests.rs"]
mod endpoint_tests;

#[cfg(test)]
#[path = "uploader_final_flush_tests.rs"]
mod final_flush_tests;

#[cfg(test)]
#[path = "uploader_tests.rs"]
mod tests;

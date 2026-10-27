//! Launcher log lines as telemetry rows, and the paced sender that posts them
//! through a `purpose = "logs"` session (LF-14; server contract C-LF4..C-LF6).
//!
//! Wire facts that shape this file:
//! - The server drops a row that breaks a C-LF5 field rule, so every string is
//!   cleaned here to the server's character set rather than left to be dropped.
//! - The install's event session shares a 20-chunks-per-minute window with the
//!   log session. Pacing (wave 3 ruling D1) keeps a full backlog under 18 chunks
//!   a minute instead of asking the server for a bigger window.
//! - Nothing here logs a path, URL, token or error text (R5).
use super::lines::LogLine;
use crate::storage::game_telemetry::ip_scrub::scrub_ips;
use crate::storage::game_telemetry::launcher_event::Redaction;
use crate::storage::game_telemetry::launcher_session::{
    BatchOutcome, LauncherSender, LauncherTarget,
};
use cimmeria_launcher_core::telemetry::chunk::{self, MAX_CHUNK_ROWS};
use cimmeria_launcher_core::telemetry::events::{ClientNativeEvent, TelemetryEvent};
use serde_json::{json, Map, Value};
use std::future::Future;
use std::ops::Range;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// The launcher's own log (C-LF5).
pub(crate) const LAUNCHER_LOG_TARGET: &str = "client.launcher.log";
/// The game's logs; LF-19 is the first sender.
pub(crate) const GAME_LOG_TARGET: &str = "client.launcher.game_log";
/// Both log targets, in the order of `launcher_log_targets.txt`.
pub(crate) const LOG_UPLOAD_TARGETS: [&str; 2] = [LAUNCHER_LOG_TARGET, GAME_LOG_TARGET];

/// Wave 3 ruling D1: at most 18 log chunks in any 60 s window.
pub(crate) const MIN_CHUNK_INTERVAL_MS: u64 = 3_500;
/// Uncompressed NDJSON per chunk; the server refuses more than 512 KiB.
const MAX_BATCH_BYTES: u64 = 384 * 1024;
/// Compressed body cap; the server refuses more than 64 KiB.
const MAX_GZIP_BYTES: usize = 64 * 1024;
/// The shortest wait a 429 or 503 reports (R16).
const MIN_RETRY_SECS: u64 = 60;
/// The server cuts a message here anyway; cutting first keeps the row small.
const MAX_MESSAGE_BYTES: usize = 2_048;
/// C-LF6: at most this many rows in one upload; the rest wait for the next press.
const MAX_UPLOAD_ROWS: usize = 40_000;
/// Planned posts per upload, so a split that blows up cannot drive many mints.
const MAX_UPLOAD_CHUNKS: usize = 400;

/// The launcher side of C-LF5. `launcher_version` and `os` are the same on every row.
/// Returns no rows when `target` is not in [`LOG_UPLOAD_TARGETS`] or `upload_id`
/// is not 16 lowercase hex digits (the server would drop each row and still
/// answer 2xx). The caller treats an empty result as `Failed` and does not
/// advance its offsets.
/// Every message is scrubbed here again, whatever made the line: a line that
/// reached this point by any route (the launcher files, the game logs) leaves
/// the machine only after the per-word scrub and the IP scrub. Scrubbing is
/// idempotent, so a line `lines::collect` already cleaned is unchanged.
pub(crate) fn rows(
    lines: &[LogLine],
    target: &'static str,
    upload_id: &str,
    launcher_version: &str,
    os: &str,
) -> Vec<TelemetryEvent> {
    if !LOG_UPLOAD_TARGETS.contains(&target) || !is_upload_id(upload_id) {
        return Vec::new();
    }
    let read_at = chrono::Utc::now().timestamp_millis();
    let version = clean(launcher_version, 32, |c| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '+' | '-')
    });
    let os = clean(&os.to_ascii_lowercase(), 16, |c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
    });
    let redaction = Redaction::from_env();
    lines
        .iter()
        .enumerate()
        .map(|(seq, line)| {
            // Game rows get the per-word scrub (A5, A9); launcher rows keep the
            // whole-value scrub.
            let scrubbed = if target == GAME_LOG_TARGET {
                redaction.scrub_words(&line.message)
            } else {
                redaction.scrub(&line.message)
            };
            let scrubbed = scrub_ips(&scrubbed).into_owned();
            let (message, cut) = cut_message(&scrubbed);
            let mut fields = Map::new();
            fields.insert("message".into(), json!(message));
            if let Some(level) = line
                .level
                .as_deref()
                .map(str::to_ascii_uppercase)
                .filter(|l| matches!(l.as_str(), "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"))
            {
                fields.insert("log_level".into(), json!(level));
            }
            if let Some(log_target) = line.target.as_deref().map(|t| clean(t, 64, is_target_char)) {
                if !log_target.is_empty() {
                    fields.insert("log_target".into(), json!(log_target));
                }
            }
            let file = clean(&line.file, 64, is_file_char);
            fields.insert(
                "file".into(),
                json!(if file.is_empty() { "unknown" } else { &file }),
            );
            fields.insert("line_no".into(), json!(line.line_no.max(1)));
            fields.insert("upload_id".into(), json!(upload_id));
            if line.truncated || cut {
                fields.insert("truncated".into(), Value::Bool(true));
            }
            if !version.is_empty() {
                fields.insert("launcher_version".into(), json!(version));
            }
            if !os.is_empty() {
                fields.insert("os".into(), json!(os));
            }
            TelemetryEvent::ClientNative(ClientNativeEvent {
                ts_ms: line.ts_ms.unwrap_or(read_at),
                seq: seq as u64,
                target: target.into(),
                // The server forces `info`; the line's own level is `log_level`.
                level: "info".into(),
                fields,
            })
        })
        .collect()
}

/// The server's C-LF5 rule for `upload_id`: 16 lowercase hex digits.
fn is_upload_id(id: &str) -> bool {
    id.len() == 16 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_target_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '.' | '-')
}

fn is_file_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')
}

/// Replace each disallowed char with `_`, then keep at most `max` bytes (the
/// result is ASCII, so any cut is a char boundary).
fn clean(value: &str, max: usize, allowed: impl Fn(char) -> bool) -> String {
    value
        .chars()
        .map(|c| if allowed(c) { c } else { '_' })
        .take(max)
        .collect()
}

fn cut_message(message: &str) -> (&str, bool) {
    if message.len() <= MAX_MESSAGE_BYTES {
        return (message, false);
    }
    let mut end = MAX_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    (&message[..end], true)
}

/// What one upload did. `lines_sent` counts rows the server accepted; a row
/// dropped as too big on its own is not in it (the finish event counts those).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IngestOutcome {
    Sent {
        lines_sent: u64,
    },
    Partial {
        lines_sent: u64,
        retry_after_secs: u64,
    },
    /// No log session could be minted for the first chunk.
    Unreachable,
    Failed,
}

/// The clock and the sleep, so a test runs 40 chunks without waiting.
pub(crate) trait Pacer {
    fn now_ms(&self) -> u64;
    fn sleep_ms(&mut self, ms: u64) -> impl Future<Output = ()> + Send;
}

/// The production pacer: `tokio::time`, so a paused test runtime also works.
#[derive(Debug)]
pub(crate) struct TokioPacer {
    origin: tokio::time::Instant,
}

impl TokioPacer {
    pub(crate) fn new() -> Self {
        Self {
            origin: tokio::time::Instant::now(),
        }
    }
}

impl Pacer for TokioPacer {
    fn now_ms(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn sleep_ms(&mut self, ms: u64) -> impl Future<Output = ()> + Send {
        tokio::time::sleep(std::time::Duration::from_millis(ms))
    }
}

/// Where a finished chunk goes. Production posts through a [`LauncherSender`];
/// pacing tests use a recording fake.
pub(crate) trait ChunkSink {
    fn post(&mut self, batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send;

    /// The scheme and host the sink last posted to, for the finish line (`None` when unknown).
    fn endpoint(&self) -> Option<(String, String)> {
        None
    }
}

struct SenderSink<'a> {
    sender: &'a LauncherSender,
    target: &'a LauncherTarget,
}

impl ChunkSink for SenderSink<'_> {
    fn post(&mut self, batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send {
        self.sender.post_batch(self.target, batch)
    }

    fn endpoint(&self) -> Option<(String, String)> {
        self.sender.last_endpoint()
    }
}

/// One planned post, or a row that cannot be sent at all.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Send(Range<usize>),
    /// A single row whose gzip body is over the cap; no split helps.
    Drop,
}

/// Split by size and rows, then halve any batch whose gzip body is over the cap
/// (log text is usually very compressible, random-looking text is not).
fn plan(events: &[TelemetryEvent]) -> Option<Vec<Step>> {
    let ranges = chunk::split_batches(events, MAX_BATCH_BYTES, MAX_CHUNK_ROWS).ok()?;
    let mut steps = Vec::new();
    for range in ranges {
        let mut stack = vec![range];
        while let Some(range) = stack.pop() {
            let body = chunk::encode_ndjson_gzip(&events[range.clone()]).ok()?;
            if body.len() <= MAX_GZIP_BYTES {
                steps.push(Step::Send(range));
            } else if range.len() <= 1 {
                steps.push(Step::Drop);
            } else {
                let mid = range.start + range.len() / 2;
                // Pop order: the first half goes out first.
                stack.push(mid..range.end);
                stack.push(range.start..mid);
            }
        }
    }
    Some(steps)
}

/// Send `events` through `sender`'s log session. See [`send_with`].
pub(crate) async fn send_logs(
    sender: &LauncherSender,
    target: &LauncherTarget,
    events: &[TelemetryEvent],
    pacer: &mut impl Pacer,
    cancel: &CancellationToken,
    on_chunk_sent: impl FnMut(usize),
) -> IngestOutcome {
    send_with(
        &mut SenderSink { sender, target },
        events,
        pacer,
        cancel,
        on_chunk_sent,
    )
    .await
}

/// Post `events` in order, one chunk at a time, `MIN_CHUNK_INTERVAL_MS` apart
/// (measured start to start; no sleep when the last post already took longer).
/// `on_chunk_sent` gets the number of rows the caller may mark done: the chunk's
/// rows plus any too-big rows skipped just before it (and after the last chunk),
/// so an offset never sticks on a row that can never be sent. A 429 or 503 ends
/// the upload as `Partial`; there is no retry loop (R16). More than
/// `MAX_UPLOAD_ROWS` rows or `MAX_UPLOAD_CHUNKS` posts are cut: the first ones are
/// sent and the upload ends `Partial` with a 60 s wait, the rest go on the next press.
/// `cancel` is checked before each paced sleep and again before each post (the
/// sleep itself also ends on a cancel): a cancel ends the upload as `Partial` with
/// the rows already sent, and no further post is made.
pub(crate) async fn send_with(
    sink: &mut impl ChunkSink,
    events: &[TelemetryEvent],
    pacer: &mut impl Pacer,
    cancel: &CancellationToken,
    mut on_chunk_sent: impl FnMut(usize),
) -> IngestOutcome {
    log_started(events.len());
    let rows_capped = events.len() > MAX_UPLOAD_ROWS;
    // Planning gzips every batch, so it runs off the async worker threads.
    let events: Arc<[TelemetryEvent]> = events[..events.len().min(MAX_UPLOAD_ROWS)].into();
    let planning = Arc::clone(&events);
    let Ok(Some(steps)) = tokio::task::spawn_blocking(move || plan(&planning)).await else {
        log_finished("failed", "plan_failed", 0, 0, 0, sink.endpoint());
        return IngestOutcome::Failed;
    };
    let (steps, chunks_capped) = cap_sends(steps);
    let capped = rows_capped || chunks_capped;
    let mut last_start: Option<u64> = None;
    let (mut lines_sent, mut chunks, mut dropped) = (0u64, 0u64, 0u64);
    let mut skipped = 0usize;
    for step in steps {
        let range = match step {
            Step::Drop => {
                dropped += 1;
                skipped += 1;
                continue;
            }
            Step::Send(range) => range,
        };
        if cancel.is_cancelled() {
            return cancelled(lines_sent, chunks, dropped, sink.endpoint());
        }
        if let Some(previous) = last_start {
            let since = pacer.now_ms().saturating_sub(previous);
            if since < MIN_CHUNK_INTERVAL_MS {
                tokio::select! {
                    () = pacer.sleep_ms(MIN_CHUNK_INTERVAL_MS - since) => {}
                    () = cancel.cancelled() => {}
                }
            }
        }
        // Checked again: the sleep may have ended because of the cancel.
        if cancel.is_cancelled() {
            return cancelled(lines_sent, chunks, dropped, sink.endpoint());
        }
        last_start = Some(pacer.now_ms());
        let rows = range.len();
        let (outcome, reason) = match sink.post(&events[range]).await {
            BatchOutcome::Posted => {
                chunks += 1;
                lines_sent += rows as u64;
                on_chunk_sent(rows + std::mem::take(&mut skipped));
                continue;
            }
            BatchOutcome::Throttled { retry_after_secs } => (
                IngestOutcome::Partial {
                    lines_sent,
                    retry_after_secs: retry_after_secs.max(MIN_RETRY_SECS),
                },
                "throttled",
            ),
            BatchOutcome::MintFailed { reason } if chunks == 0 => {
                (IngestOutcome::Unreachable, reason)
            }
            BatchOutcome::MintFailed { reason } => (IngestOutcome::Failed, reason),
            BatchOutcome::Failed => (IngestOutcome::Failed, "post_failed"),
        };
        log_finished(
            outcome_name(outcome),
            reason,
            lines_sent,
            chunks,
            dropped,
            sink.endpoint(),
        );
        return outcome;
    }
    if skipped > 0 {
        on_chunk_sent(skipped);
    }
    if capped {
        log_finished(
            "partial",
            "upload_cap",
            lines_sent,
            chunks,
            dropped,
            sink.endpoint(),
        );
        return IngestOutcome::Partial {
            lines_sent,
            retry_after_secs: MIN_RETRY_SECS,
        };
    }
    log_finished("sent", "none", lines_sent, chunks, dropped, sink.endpoint());
    IngestOutcome::Sent { lines_sent }
}

/// A cancel keeps what was sent; the caller's offsets already stop at the last chunk.
fn cancelled(
    lines_sent: u64,
    chunks: u64,
    dropped: u64,
    endpoint: Option<(String, String)>,
) -> IngestOutcome {
    log_finished(
        "partial",
        "cancelled",
        lines_sent,
        chunks,
        dropped,
        endpoint,
    );
    IngestOutcome::Partial {
        lines_sent,
        retry_after_secs: MIN_RETRY_SECS,
    }
}

/// Keeps the first `MAX_UPLOAD_CHUNKS` posts and says whether any were cut.
fn cap_sends(steps: Vec<Step>) -> (Vec<Step>, bool) {
    let mut sends = 0usize;
    let mut kept = Vec::with_capacity(steps.len());
    for step in steps {
        if matches!(step, Step::Send(_)) {
            if sends == MAX_UPLOAD_CHUNKS {
                return (kept, true);
            }
            sends += 1;
        }
        kept.push(step);
    }
    (kept, false)
}

fn outcome_name(outcome: IngestOutcome) -> &'static str {
    match outcome {
        IngestOutcome::Sent { .. } => "sent",
        IngestOutcome::Partial { .. } => "partial",
        IngestOutcome::Unreachable => "unreachable",
        IngestOutcome::Failed => "failed",
    }
}

/// The upload id is a correlation key for the rows only, so neither event carries it.
fn log_started(lines: usize) {
    tracing::info!(
        target: "launcher.telemetry",
        event = "log_upload_started",
        lines = lines as u64,
        "launcher log upload: sending log lines"
    );
}

/// `reason` is a closed code: `none` on success, else why the upload stopped.
/// `scheme` and `host` name the endpoint the last post went to; `-` when none did.
fn log_finished(
    outcome: &'static str,
    reason: &'static str,
    lines_sent: u64,
    chunks: u64,
    rows_dropped: u64,
    endpoint: Option<(String, String)>,
) {
    let (scheme, host) = match &endpoint {
        Some((scheme, host)) => (scheme.as_str(), host.as_str()),
        None => ("-", "-"),
    };
    tracing::info!(
        target: "launcher.telemetry",
        event = "log_upload_finished",
        outcome,
        reason,
        lines_sent,
        chunks,
        rows_dropped,
        scheme,
        host,
        "launcher log upload: finished"
    );
}

#[cfg(test)]
#[path = "ingest_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "ingest_endpoint_tests.rs"]
mod endpoint_tests;

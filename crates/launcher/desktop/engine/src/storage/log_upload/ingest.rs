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
use crate::storage::game_telemetry::launcher_session::{
    BatchOutcome, LauncherSender, LauncherTarget,
};
use cimmeria_launcher_core::telemetry::chunk::{self, MAX_CHUNK_ROWS};
use cimmeria_launcher_core::telemetry::events::{ClientNativeEvent, TelemetryEvent};
use serde_json::{json, Map, Value};
use std::future::Future;
use std::ops::Range;

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

/// The launcher side of C-LF5. `launcher_version` and `os` are the same on every row.
pub(crate) fn rows(
    lines: &[LogLine],
    target: &'static str,
    upload_id: &str,
    launcher_version: &str,
    os: &str,
) -> Vec<TelemetryEvent> {
    debug_assert!(LOG_UPLOAD_TARGETS.contains(&target));
    let read_at = chrono::Utc::now().timestamp_millis();
    let version = clean(launcher_version, 32, |c| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '+' | '-')
    });
    let os = clean(&os.to_ascii_lowercase(), 16, |c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'
    });
    lines
        .iter()
        .enumerate()
        .map(|(seq, line)| {
            let (message, cut) = cut_message(&line.message);
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
}

struct SenderSink<'a> {
    sender: &'a LauncherSender,
    target: &'a LauncherTarget,
}

impl ChunkSink for SenderSink<'_> {
    fn post(&mut self, batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send {
        self.sender.post_batch(self.target, batch)
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
    on_chunk_sent: impl FnMut(usize),
) -> IngestOutcome {
    send_with(
        &mut SenderSink { sender, target },
        events,
        pacer,
        on_chunk_sent,
    )
    .await
}

/// Post `events` in order, one chunk at a time, `MIN_CHUNK_INTERVAL_MS` apart
/// (measured start to start; no sleep when the last post already took longer).
/// `on_chunk_sent` gets the number of rows the caller may mark done: the chunk's
/// rows plus any too-big rows skipped just before it (and after the last chunk),
/// so an offset never sticks on a row that can never be sent. A 429 or 503 ends
/// the upload as `Partial`; there is no retry loop (R16).
pub(crate) async fn send_with(
    sink: &mut impl ChunkSink,
    events: &[TelemetryEvent],
    pacer: &mut impl Pacer,
    mut on_chunk_sent: impl FnMut(usize),
) -> IngestOutcome {
    log_started(events.len());
    let Some(steps) = plan(events) else {
        log_finished("failed", 0, 0, 0);
        return IngestOutcome::Failed;
    };
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
        if let Some(previous) = last_start {
            let since = pacer.now_ms().saturating_sub(previous);
            if since < MIN_CHUNK_INTERVAL_MS {
                pacer.sleep_ms(MIN_CHUNK_INTERVAL_MS - since).await;
            }
        }
        last_start = Some(pacer.now_ms());
        let rows = range.len();
        let outcome = match sink.post(&events[range]).await {
            BatchOutcome::Posted => {
                chunks += 1;
                lines_sent += rows as u64;
                on_chunk_sent(rows + std::mem::take(&mut skipped));
                continue;
            }
            BatchOutcome::Throttled { retry_after_secs } => IngestOutcome::Partial {
                lines_sent,
                retry_after_secs: retry_after_secs.max(MIN_RETRY_SECS),
            },
            BatchOutcome::MintFailed { .. } if chunks == 0 => IngestOutcome::Unreachable,
            BatchOutcome::MintFailed { .. } | BatchOutcome::Failed => IngestOutcome::Failed,
        };
        log_finished(outcome_name(outcome), lines_sent, chunks, dropped);
        return outcome;
    }
    if skipped > 0 {
        on_chunk_sent(skipped);
    }
    log_finished("sent", lines_sent, chunks, dropped);
    IngestOutcome::Sent { lines_sent }
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

fn log_finished(outcome: &'static str, lines_sent: u64, chunks: u64, rows_dropped: u64) {
    tracing::info!(
        target: "launcher.telemetry",
        event = "log_upload_finished",
        outcome,
        lines_sent,
        chunks,
        rows_dropped,
        "launcher log upload: finished"
    );
}

#[cfg(test)]
#[path = "ingest_tests.rs"]
mod tests;

//! Per-session telemetry loop.
//!
//! Spawned by the worker once the game is alive: ticks every
//! `flush_interval_ms`, drains [`super::tail::Tailer`] for any new
//! lines under `<install_dir>/Binaries/sessions/`, converts each
//! line to a [`TelemetryEvent`], enqueues, and flushes. When the
//! game exits the loop does one final tick + flush, builds the
//! end-of-session bundle, and reports the outcome back to the
//! worker.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::time::interval;

use super::events::{
    parse_client_log_line, ClientLogEvent, DebugLogEvent, SessionMetaEvent, SessionMetaKind,
    TelemetryEvent,
};
use super::patch_log::PatchLogWatcher;
use super::process_watch::{ExitWaiter, WatchError};
use super::queue::DiskQueue;
use super::tail::{TailedLine, Tailer};
use super::{Telemetry, TelemetryError};

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error(transparent)]
    Telemetry(#[from] TelemetryError),
    #[error(transparent)]
    Watch(#[from] WatchError),
}

/// Outcome reported back to the worker once a telemetry session ends.
/// Fields are oriented around what the UI surfaces ("uploaded N
/// events, M bytes") rather than internal counters.
#[derive(Debug, Clone)]
pub struct SessionOutcome {
    pub event_count: u64,
    pub dropped_lines: u64,
    pub bundle_bytes: u64,
    pub bundle_sha256: String,
}

/// Run a telemetry session to completion.
///
/// 1. Tail the sessions dir + sgwdebuglog* alongside the game.
/// 2. Tick at `flush_interval_ms` cadence: parse new lines, enqueue,
///    POST chunk. With `patch_log`, also record the client-patches
///    DLL's boot summary once (see [`super::patch_log`]).
/// 3. When the game exits: final tick, final flush, bundle upload.
///
/// Drop-safety: `exit` waits on the game without owning its lifetime
/// (see [`super::process_watch`]). If the launcher dies before this
/// future resolves, the game keeps running and the bundle just doesn't
/// upload (the on-disk queue picks it up on the next launcher
/// startup via `recover_pending_on_startup`).
pub async fn run_session(
    telemetry: Arc<Telemetry>,
    http: Arc<reqwest::Client>,
    exit: ExitWaiter,
    install_dir: PathBuf,
    state_dir: PathBuf,
    mut patch_log: Option<PatchLogWatcher>,
) -> Result<SessionOutcome, RunnerError> {
    let flush_ms = {
        let s = telemetry.session.read().await;
        s.flush_interval_ms.max(250)
    };
    let mut ticker = interval(Duration::from_millis(flush_ms));
    let mut tailer = Tailer::new();
    let binaries_dir = crate::install_layout::binaries_dir(&install_dir);
    let sessions_dir = binaries_dir.join("sessions");

    let _ = telemetry
        .enqueue(TelemetryEvent::SessionMeta(SessionMetaEvent {
            ts_ms: 0,
            seq: 0,
            kind: SessionMetaKind::Started,
            fields: serde_json::Map::new(),
        }))
        .await;

    let mut event_count: u64 = 0;
    let waiter = tokio::spawn(exit);
    tokio::pin!(waiter);

    loop {
        tokio::select! {
            biased;
            res = &mut waiter => {
                let exit = res.map_err(|_| WatchError::JoinPanic)??;
                tracing::info!(
                    pid = exit.pid,
                    exit_code = ?exit.exit_code,
                    "game process exited — running final telemetry flush"
                );
                break;
            }
            _ = ticker.tick() => {
                let patch_event = patch_log.as_mut().and_then(PatchLogWatcher::poll);
                event_count = event_count.saturating_add(enqueue_patch_event(&telemetry, patch_event).await);
                event_count = event_count.saturating_add(
                    tick_once(&telemetry, &http, &mut tailer, &sessions_dir, &binaries_dir).await,
                );
            }
        }
    }

    let patch_event = patch_log.as_mut().and_then(PatchLogWatcher::finish);
    event_count = event_count.saturating_add(enqueue_patch_event(&telemetry, patch_event).await);
    event_count = event_count.saturating_add(
        tick_once(&telemetry, &http, &mut tailer, &sessions_dir, &binaries_dir).await,
    );
    let queue = DiskQueue::new(&state_dir);
    let dropped_lines = queue.dropped_count();
    let outcome = match telemetry
        .upload_bundle(&http, event_count, dropped_lines)
        .await
    {
        Ok(b) => {
            let _ = queue.dropped_counter_reset();
            SessionOutcome {
                event_count,
                dropped_lines,
                bundle_bytes: b.zip_bytes,
                bundle_sha256: b.zip_sha256,
            }
        }
        Err(TelemetryError::Bundle(super::bundle::BundleError::Empty)) => SessionOutcome {
            event_count,
            dropped_lines,
            bundle_bytes: 0,
            bundle_sha256: String::new(),
        },
        Err(e) => return Err(e.into()),
    };
    Ok(outcome)
}

/// Enqueue the client-patches boot summary, if this tick produced it.
/// Returns the number of events enqueued (0 or 1).
async fn enqueue_patch_event(telemetry: &Telemetry, event: Option<TelemetryEvent>) -> u64 {
    let Some(event) = event else {
        return 0;
    };
    match telemetry.enqueue(event).await {
        Ok(()) => 1,
        Err(e) => {
            tracing::warn!(error = %e, "client-patches boot summary not queued");
            0
        }
    }
}

/// One refresh + tick + parse + enqueue + flush cycle. Returns the
/// number of events enqueued this tick (not the number of bytes
/// flushed) so the caller can keep a running event_count for the
/// final bundle metadata.
async fn tick_once(
    telemetry: &Telemetry,
    http: &reqwest::Client,
    tailer: &mut Tailer,
    sessions_dir: &Path,
    binaries_dir: &Path,
) -> u64 {
    if sessions_dir.is_dir() {
        tailer.refresh(sessions_dir, |p| {
            p.extension().is_some_and(|e| e == "log" || e == "txt")
        });
    }
    if binaries_dir.is_dir() {
        tailer.refresh(binaries_dir, |p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(is_debug_log)
        });
    }
    let lines = tailer.tick();
    let line_count = lines.len();
    let mut enqueued = 0u64;
    for tl in lines {
        let ev = line_to_event(tl);
        if telemetry.enqueue(ev).await.is_ok() {
            enqueued += 1;
        }
    }
    if line_count > 0 {
        tracing::debug!(
            watched_files = tailer.watched_count(),
            lines = line_count,
            enqueued,
            "telemetry tail tick"
        );
    }
    // Proactive token refresh before the chunk POST so an expired
    // token never causes the first failed-flush of the session.
    if let Err(e) = telemetry.refresh_if_due(http).await {
        tracing::warn!(error = %e, "proactive token refresh failed; continuing on current token");
    }
    if let Err(e) = telemetry.flush(http).await {
        tracing::warn!(error = %e, "telemetry chunk flush failed; events queued for retry");
    }
    enqueued
}

/// True for the client's BigWorld debug log. The client (and patch
/// `004-log-config`) writes `SGWDebugLog.log`; the match ignores case,
/// since a lowercase-only match never tailed it.
fn is_debug_log(file_name: &str) -> bool {
    file_name.to_ascii_lowercase().starts_with("sgwdebuglog")
}

/// Convert a [`TailedLine`] to a [`TelemetryEvent`]. `sgwdebuglog*`
/// goes to `DebugLog`; everything else is treated as an Atera client
/// log line and run through [`parse_client_log_line`].
fn line_to_event(tl: TailedLine) -> TelemetryEvent {
    if is_debug_log(&tl.source_file) {
        TelemetryEvent::DebugLog(DebugLogEvent {
            ts_ms: 0,
            seq: 0,
            source_file: tl.source_file,
            level: "info".into(),
            message: tl.line,
        })
    } else {
        let parsed = parse_client_log_line(&tl.line);
        TelemetryEvent::ClientLog(ClientLogEvent {
            ts_ms: 0,
            seq: 0,
            source_file: tl.source_file,
            level: parsed.level.to_string(),
            category: parsed.category.to_string(),
            packet_no: parsed.packet_no,
            message: parsed.message.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::tail::TailedLine;

    #[test]
    fn line_to_event_routes_sgwdebuglog_to_debug_log() {
        let ev = line_to_event(TailedLine {
            source_file: "sgwdebuglog".into(),
            line: "anything".into(),
        });
        match ev {
            TelemetryEvent::DebugLog(d) => assert_eq!(d.message, "anything"),
            other => panic!("expected DebugLog, got {other:?}"),
        }
    }

    /// The name the client actually writes. A lowercase-only match
    /// skipped it, so no debug-log line was ever uploaded.
    #[test]
    fn the_clients_mixed_case_debug_log_is_a_debug_log() {
        assert!(is_debug_log("SGWDebugLog.log"));
        assert!(is_debug_log("sgwdebuglog.log.1"));
        assert!(!is_debug_log("SGWLogConfig.xml"));
        assert!(!is_debug_log("SGW.exe"));
        let ev = line_to_event(TailedLine {
            source_file: "SGWDebugLog.log".into(),
            line: "2026-09-28 21:07:21,612 DEBUG common - about to call _runThread".into(),
        });
        assert!(matches!(ev, TelemetryEvent::DebugLog(_)), "{ev:?}");
    }

    #[test]
    fn line_to_event_parses_atera_format_into_client_log() {
        let ev = line_to_event(TailedLine {
            source_file: "2026-05-25.log".into(),
            line: "[t] [warn] [Mercury] retransmit pkt=42".into(),
        });
        match ev {
            TelemetryEvent::ClientLog(c) => {
                assert_eq!(c.level, "warn");
                assert_eq!(c.category, "Mercury");
                assert_eq!(c.packet_no, Some(42));
            }
            other => panic!("expected ClientLog, got {other:?}"),
        }
    }
}

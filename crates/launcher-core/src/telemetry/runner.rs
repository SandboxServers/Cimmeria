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
use super::{Telemetry, TelemetryError, MAX_CHUNKS_PER_FLUSH};

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
///    DLL's boot summary once and its claimed / delivered / dropped
///    counts (see [`super::patch_log`], [`super::patch_counts`]).
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

    enqueue_session_meta(&telemetry, SessionMetaKind::Started, serde_json::Map::new()).await;

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
                // Queued before the final flush so the exit reaches the
                // server with the last chunk.
                let mut fields = serde_json::Map::new();
                fields.insert("pid".into(), serde_json::Value::from(exit.pid));
                if let Some(code) = exit.exit_code {
                    fields.insert("exit_code".into(), serde_json::Value::from(code));
                }
                enqueue_session_meta(&telemetry, SessionMetaKind::GameExited, fields).await;
                break;
            }
            _ = ticker.tick() => {
                let patch_event = patch_log.as_mut().and_then(PatchLogWatcher::poll);
                event_count = event_count.saturating_add(enqueue_patch_event(&telemetry, patch_event).await);
                let now = std::time::Instant::now();
                let counts_event = patch_log.as_mut().and_then(|w| w.poll_counts(now));
                event_count = event_count.saturating_add(enqueue_patch_event(&telemetry, counts_event).await);
                event_count = event_count.saturating_add(
                    tick_once(&telemetry, &http, &mut tailer, &binaries_dir, MAX_CHUNKS_PER_FLUSH).await,
                );
            }
        }
    }

    let patch_event = patch_log.as_mut().and_then(PatchLogWatcher::finish);
    event_count = event_count.saturating_add(enqueue_patch_event(&telemetry, patch_event).await);
    let counts_event = patch_log.as_mut().and_then(PatchLogWatcher::finish_counts);
    event_count = event_count.saturating_add(enqueue_patch_event(&telemetry, counts_event).await);
    event_count = event_count
        .saturating_add(tick_once(&telemetry, &http, &mut tailer, &binaries_dir, usize::MAX).await);
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

/// Enqueue a launcher-side `session_meta` row. A failed enqueue is logged
/// at WARN: the row is lost, but the session carries on.
async fn enqueue_session_meta(
    telemetry: &Telemetry,
    kind: SessionMetaKind,
    fields: serde_json::Map<String, serde_json::Value>,
) {
    let ev = TelemetryEvent::SessionMeta(SessionMetaEvent {
        ts_ms: 0,
        seq: 0,
        kind,
        fields,
    });
    if let Err(e) = telemetry.enqueue(ev).await {
        tracing::warn!(error = %e, "session_meta event not queued");
    }
}

/// Enqueue a client-patches event (boot summary or counts), if this tick
/// produced one.
/// Returns the number of events enqueued (0 or 1).
async fn enqueue_patch_event(telemetry: &Telemetry, event: Option<TelemetryEvent>) -> u64 {
    let Some(event) = event else {
        return 0;
    };
    match telemetry.enqueue(event).await {
        Ok(()) => 1,
        Err(e) => {
            tracing::warn!(error = %e, "client-patches event not queued");
            0
        }
    }
}

/// One refresh + tick + parse + enqueue + flush cycle. Returns the
/// number of events enqueued this tick (not the number of bytes
/// flushed) so the caller can keep a running event_count for the
/// final bundle metadata. The final tick at game exit passes
/// `usize::MAX` chunks so the whole backlog goes before the bundle.
async fn tick_once(
    telemetry: &Telemetry,
    http: &reqwest::Client,
    tailer: &mut Tailer,
    binaries_dir: &Path,
    max_chunks: usize,
) -> u64 {
    refresh_tail(tailer, binaries_dir);
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
    if let Err(e) = telemetry.flush(http, max_chunks).await {
        tracing::warn!(error = %e, "telemetry chunk flush failed; events queued for retry");
    }
    enqueued
}

/// Pick up the session's files. One walk with one predicate: `refresh`
/// keeps only what its own walk matched, so the two walks this used to
/// make (`sessions/`, then `Binaries/` for the debug log) each dropped the
/// other's files and re-added them at their end, and no line shipped.
fn refresh_tail(tailer: &mut Tailer, binaries_dir: &Path) {
    tailer.refresh(binaries_dir, |p| is_tailed(binaries_dir, p));
}

/// True for a file the session tail follows: the client's debug log
/// anywhere under `Binaries`, and every `.log` / `.txt` under
/// `Binaries/sessions` at any depth (the files the end-of-session bundle
/// takes). Case is ignored throughout; Windows paths are case-insensitive.
pub fn is_tailed(binaries_dir: &Path, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if is_debug_log(name) {
        return true;
    }
    let Ok(relative) = path.strip_prefix(binaries_dir) else {
        return false;
    };
    let mut parts = relative.components();
    let in_sessions = parts
        .next()
        .and_then(|first| first.as_os_str().to_str())
        .is_some_and(|first| first.eq_ignore_ascii_case("sessions"))
        && parts.next().is_some();
    in_sessions
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("log") || ext.eq_ignore_ascii_case("txt"))
}

/// True for the client's BigWorld debug log. The client (and patch
/// `004-log-config`) writes `SGWDebugLog.log`; the match ignores case,
/// since a lowercase-only match never tailed it.
pub fn is_debug_log(file_name: &str) -> bool {
    file_name.to_ascii_lowercase().starts_with("sgwdebuglog")
}

/// Convert a [`TailedLine`] to a [`TelemetryEvent`]. `sgwdebuglog*`
/// goes to `DebugLog`; everything else is treated as an Atera client
/// log line and run through [`parse_client_log_line`].
pub fn line_to_event(tl: TailedLine) -> TelemetryEvent {
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

    /// A game exit is queued as `session_meta kind=game_exited` carrying its pid
    /// and exit code. Removing the enqueue leaves only the `started` row.
    #[tokio::test]
    async fn game_exit_is_queued_as_session_meta_game_exited() {
        use crate::telemetry::auth::DevSessionResponse;
        use crate::telemetry::endpoint::EndpointPolicy;
        use crate::telemetry::process_watch::ExitReport;
        use crate::telemetry::queue::DiskQueue;
        use crate::telemetry::SessionContext;

        let dir = tempfile::tempdir().unwrap();
        let telemetry = Telemetry::from_minted(
            DevSessionResponse {
                session_id: "sid-1".into(),
                token: "tok".into(),
                expires_at_ms: 4_102_444_800_000,
                upload_endpoint: "http://localhost:9/api".into(),
                chunk_max_bytes: 1_048_576,
                flush_interval_ms: 2_000,
            },
            SessionContext {
                auth_base_url: "http://localhost:9/api".into(),
                install_dir: dir.path().to_path_buf(),
                install_id: "i".into(),
                machine_id: "m".into(),
                launcher_version: "0.1.0".into(),
                session_started_at_ms: 1_700_000_000_000,
                endpoint_policy: EndpointPolicy,
                queue_dir: dir.path().to_path_buf(),
            },
        )
        .unwrap();
        let exit: ExitWaiter = Box::pin(async {
            Ok(ExitReport {
                pid: 4242,
                exit_code: Some(7),
            })
        });
        // Nothing listens on localhost:9, so the final flush fails and the
        // queued rows stay on disk for the assertions below.
        let _ = run_session(
            Arc::new(telemetry),
            Arc::new(reqwest::Client::new()),
            exit,
            dir.path().to_path_buf(),
            dir.path().to_path_buf(),
            None,
        )
        .await;

        let rows = DiskQueue::new(dir.path())
            .drain::<TelemetryEvent>()
            .unwrap();
        let ended = rows
            .iter()
            .find_map(|ev| match ev {
                TelemetryEvent::SessionMeta(m) if m.kind == SessionMetaKind::GameExited => Some(m),
                _ => None,
            })
            .expect("a game_exited session_meta row");
        assert_eq!(ended.fields["pid"], 4242);
        assert_eq!(ended.fields["exit_code"], 7);
    }

    fn append(path: &Path, text: &str) {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
    }

    /// `tick_once`'s refresh, tick after tick, with both the debug log
    /// and a session log growing: lines from both arrive. The two
    /// refreshes it used to make dropped each other's files, so whenever
    /// `sessions/` existed, neither file shipped a line.
    #[test]
    fn the_tail_ships_the_debug_log_and_session_logs_together() {
        let dir = tempfile::tempdir().unwrap();
        let binaries = dir.path().join("Binaries");
        std::fs::create_dir_all(binaries.join("sessions/sub")).unwrap();
        let debug = binaries.join("SGWDebugLog.log");
        let session = binaries.join("sessions/sub/2026-10-11.log");
        append(&debug, "before\n");
        append(&session, "before\n");
        let mut tailer = Tailer::new();
        let mut lines = Vec::new();
        for round in 0..3 {
            refresh_tail(&mut tailer, &binaries);
            lines.extend(tailer.tick().into_iter().map(|l| (l.source_file, l.line)));
            append(&debug, &format!("debug {round}\n"));
            append(&session, &format!("session {round}\n"));
        }
        refresh_tail(&mut tailer, &binaries);
        lines.extend(tailer.tick().into_iter().map(|l| (l.source_file, l.line)));
        lines.sort();
        let expected: Vec<(String, String)> = (0..3)
            .map(|n| ("2026-10-11.log".to_string(), format!("session {n}")))
            .chain((0..3).map(|n| ("SGWDebugLog.log".to_string(), format!("debug {n}"))))
            .collect();
        assert_eq!(lines, expected);
    }

    /// The bundle's files, at any depth under `sessions`, in any case.
    #[test]
    fn the_tail_takes_session_logs_at_any_depth_and_in_any_case() {
        let binaries = Path::new("C:/game/Binaries");
        for tailed in [
            "SGWDebugLog.log",
            "logs/sgwdebuglog_old.log",
            "sessions/a.log",
            "sessions/b.TXT",
            "sessions/sub/deeper/c.log",
            "Sessions/d.Log",
        ] {
            assert!(is_tailed(binaries, &binaries.join(tailed)), "{tailed}");
        }
        for other in [
            "sessions/a.json",
            "sessions.log",
            "other/a.log",
            "a.log",
            "../sessions/a.log",
        ] {
            assert!(!is_tailed(binaries, &binaries.join(other)), "{other}");
        }
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

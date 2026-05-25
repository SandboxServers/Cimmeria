//! Dev-session telemetry pipeline.

// The `Telemetry` orchestrator and its submodules are wired by the
// worker in the next commit (process-watch + game-launch hook).
// Allowed here only until that wiring lands; the allow comes off in
// the same commit that adds the call site.
#![allow(dead_code)]

pub mod auth;
pub mod bundle;
pub mod chunk;
pub mod events;
pub mod queue;
pub mod session;
pub mod tail;

use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::Mutex;

use crate::config::exe_dir;
use auth::{DevSessionRequest, DevSessionResponse};
use chunk::ChunkError;
use events::TelemetryEvent;
use queue::DiskQueue;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error(transparent)]
    Auth(#[from] auth::AuthError),
    #[error(transparent)]
    Chunk(#[from] chunk::ChunkError),
    #[error(transparent)]
    Bundle(#[from] bundle::BundleError),
    #[error(transparent)]
    Session(#[from] session::SessionError),
    #[error(transparent)]
    Queue(#[from] queue::QueueError),
}

/// Live telemetry session. Holds the auth token, the disk queue for
/// overflow, and the running monotonic `seq` counter handed to each
/// event. Constructed by [`Telemetry::start_session`] after a
/// successful auth handshake; dropped when the game exits and the
/// bundle has been flushed.
pub struct Telemetry {
    pub session: DevSessionResponse,
    pub auth_base_url: String,
    pub install_dir: PathBuf,
    pub install_id: String,
    pub machine_id: String,
    pub launcher_version: String,
    pub session_started_at_ms: i64,
    queue: DiskQueue,
    seq: Arc<Mutex<u64>>,
}

impl Telemetry {
    /// One-call session bootstrap: handshake with `auth_base_url`,
    /// write `current-session.json`, return a live `Telemetry`
    /// handle. Callers feed events through [`Telemetry::enqueue`] and
    /// invoke [`Telemetry::flush`] on the flush cadence.
    pub async fn start_session(
        http: &reqwest::Client,
        auth_base_url: &str,
        req: DevSessionRequest,
        install_dir: &std::path::Path,
        launcher_version: &str,
    ) -> Result<Self, TelemetryError> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let resp = auth::fetch_dev_session(http, auth_base_url, &req).await?;
        let current = session::CurrentSession {
            schema_version: session::CURRENT_SESSION_SCHEMA,
            install_id: req.install_id.clone(),
            machine_id: req.machine_id.clone(),
            session_id: resp.session_id.clone(),
            session_started_at_ms: now_ms,
            branch: req.branch.clone(),
            git_sha: req.git_sha.clone(),
            telemetry: session::TelemetryBlock {
                enabled: true,
                token: resp.token.clone(),
                expires_at_ms: resp.expires_at_ms,
                upload_endpoint: resp.upload_endpoint.clone(),
                chunk_max_bytes: resp.chunk_max_bytes,
                flush_interval_ms: resp.flush_interval_ms,
            },
            tags: req.tags.clone(),
        };
        session::write_current_session(install_dir, &current)?;
        Ok(Self {
            session: resp,
            auth_base_url: auth_base_url.to_string(),
            install_dir: install_dir.to_path_buf(),
            install_id: req.install_id,
            machine_id: req.machine_id,
            launcher_version: launcher_version.to_string(),
            session_started_at_ms: now_ms,
            queue: DiskQueue::new(&exe_dir()),
            seq: Arc::new(Mutex::new(0)),
        })
    }

    /// Stamp `ts_ms` + `seq` onto an event and persist it to the
    /// on-disk queue. The chunk uploader drains the queue on its
    /// next tick.
    pub async fn enqueue(&self, mut ev: TelemetryEvent) -> Result<(), TelemetryError> {
        let mut seq = self.seq.lock().await;
        *seq = seq.saturating_add(1);
        stamp_seq(&mut ev, *seq);
        self.queue.enqueue(&ev)?;
        Ok(())
    }

    /// Drain the queue and POST one chunk. On chunk failure the
    /// events are re-enqueued so the next flush replays them. A
    /// `TokenRejected` propagates so the caller can run refresh +
    /// retry; other errors propagate as-is.
    pub async fn flush(&self, http: &reqwest::Client) -> Result<u64, TelemetryError> {
        let events: Vec<TelemetryEvent> = self.queue.drain()?;
        if events.is_empty() {
            return Ok(0);
        }
        let n = events.len() as u64;
        match chunk::post_chunk(
            http,
            &self.session.upload_endpoint,
            &self.session.token,
            &events,
        )
        .await
        {
            Ok(()) => Ok(n),
            Err(e) => {
                // Best-effort re-enqueue; failures here surface to
                // the tracing log but don't override the underlying
                // chunk error.
                for ev in &events {
                    if let Err(re) = self.queue.enqueue(ev) {
                        tracing::warn!(error = %re, "telemetry re-enqueue after chunk failure");
                        break;
                    }
                }
                Err(map_chunk_err(e))
            }
        }
    }

    /// Build + POST the end-of-session bundle. Caller-set counters
    /// (event_count, dropped_lines) are echoed back into metadata so
    /// the server can correlate streaming-time totals with the
    /// bundle.
    pub async fn upload_bundle(
        &self,
        http: &reqwest::Client,
        event_count: u64,
        dropped_lines: u64,
    ) -> Result<bundle::BundleOutcome, TelemetryError> {
        let metadata = bundle::BundleMetadata {
            session_id: self.session.session_id.clone(),
            install_id: self.install_id.clone(),
            machine_id: self.machine_id.clone(),
            launcher_version: self.launcher_version.clone(),
            session_started_at_ms: self.session_started_at_ms,
            session_ended_at_ms: chrono::Utc::now().timestamp_millis(),
            event_count,
            dropped_lines,
            zip_sha256: String::new(),
            zip_bytes: 0,
        };
        Ok(bundle::upload_bundle(
            http,
            &self.session.upload_endpoint,
            &self.session.token,
            &self.install_dir,
            metadata,
        )
        .await?)
    }
}

fn stamp_seq(ev: &mut TelemetryEvent, seq: u64) {
    let now_ms = chrono::Utc::now().timestamp_millis();
    match ev {
        TelemetryEvent::ClientLog(e) => {
            e.seq = seq;
            if e.ts_ms == 0 {
                e.ts_ms = now_ms;
            }
        }
        TelemetryEvent::DebugLog(e) => {
            e.seq = seq;
            if e.ts_ms == 0 {
                e.ts_ms = now_ms;
            }
        }
        TelemetryEvent::KeyDump(e) => {
            e.seq = seq;
            if e.ts_ms == 0 {
                e.ts_ms = now_ms;
            }
        }
        TelemetryEvent::SessionMeta(e) => {
            e.seq = seq;
            if e.ts_ms == 0 {
                e.ts_ms = now_ms;
            }
        }
    }
}

fn map_chunk_err(e: ChunkError) -> TelemetryError {
    TelemetryError::Chunk(e)
}

/// Drain any telemetry events left on disk from a previous launcher
/// run that crashed or was killed before its bundle could upload.
pub fn recover_pending_on_startup() -> u64 {
    let q = DiskQueue::new(&exe_dir());
    recover_pending_at(&q)
}

fn recover_pending_at(q: &DiskQueue) -> u64 {
    match q.drain::<TelemetryEvent>() {
        Ok(events) if events.is_empty() => 0,
        Ok(events) => {
            let n = events.len() as u64;
            tracing::info!(
                pending = n,
                dropped_since_last_drain = q.dropped_count(),
                "Recovered telemetry events from previous session — will replay on next flush"
            );
            for ev in &events {
                if let Err(e) = q.enqueue(ev) {
                    tracing::warn!(error = %e, "failed to re-enqueue recovered event");
                    break;
                }
            }
            n
        }
        Err(e) => {
            tracing::warn!(error = %e, "telemetry recovery drain failed");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::events::{ClientLogEvent, TelemetryEvent};

    #[test]
    fn recover_pending_returns_zero_on_empty_queue() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        assert_eq!(recover_pending_at(&q), 0);
    }

    #[test]
    fn recover_pending_drains_and_reenqueues() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        for i in 0..3 {
            q.enqueue(&TelemetryEvent::ClientLog(ClientLogEvent {
                ts_ms: i,
                seq: i as u64,
                source_file: "x.log".into(),
                level: "info".into(),
                category: "raw".into(),
                packet_no: None,
                message: format!("e-{i}"),
            }))
            .unwrap();
        }
        let n = recover_pending_at(&q);
        assert_eq!(n, 3);
        let drained: Vec<TelemetryEvent> = q.drain().unwrap();
        assert_eq!(drained.len(), 3);
    }

    // stamp_seq rewrites the seq field on every variant and fills in
    // ts_ms when zero. Pinned because the orchestrator's monotonic
    // ordering guarantee depends on it.
    #[test]
    fn stamp_seq_writes_seq_on_every_variant() {
        let cases = vec![
            TelemetryEvent::ClientLog(ClientLogEvent {
                ts_ms: 0,
                seq: 0,
                source_file: "x".into(),
                level: "i".into(),
                category: "c".into(),
                packet_no: None,
                message: "m".into(),
            }),
            TelemetryEvent::DebugLog(events::DebugLogEvent {
                ts_ms: 0,
                seq: 0,
                source_file: "x".into(),
                level: "i".into(),
                message: "m".into(),
            }),
            TelemetryEvent::KeyDump(events::KeyDumpEvent {
                ts_ms: 0,
                seq: 0,
                source_file: "x".into(),
                key_b64: "k".into(),
            }),
            TelemetryEvent::SessionMeta(events::SessionMetaEvent {
                ts_ms: 0,
                seq: 0,
                kind: events::SessionMetaKind::Started,
                fields: serde_json::Map::new(),
            }),
        ];
        for mut ev in cases {
            stamp_seq(&mut ev, 42);
            let seq_after = match &ev {
                TelemetryEvent::ClientLog(e) => e.seq,
                TelemetryEvent::DebugLog(e) => e.seq,
                TelemetryEvent::KeyDump(e) => e.seq,
                TelemetryEvent::SessionMeta(e) => e.seq,
            };
            assert_eq!(seq_after, 42);
        }
    }

    // start_session writes current-session.json + returns a Telemetry
    // whose enqueue / flush round-trip through the disk queue and the
    // chunk endpoint.
    #[tokio::test]
    async fn start_session_writes_marker_then_enqueue_flush_round_trips() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sid-1",
                "token": "tok",
                "expires_at_ms": 1_700_028_800_000_i64,
                "upload_endpoint": format!("{}/api", server.uri()),
                "chunk_max_bytes": 1_048_576,
                "flush_interval_ms": 2_000,
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/upload-chunk"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let http = reqwest::Client::new();
        let tel = Telemetry::start_session(
            &http,
            &server.uri(),
            DevSessionRequest {
                install_id: "i".into(),
                machine_id: "m".into(),
                branch: "main".into(),
                git_sha: "abc".into(),
                launcher_version: "0.1.0".into(),
                tags: vec![],
            },
            dir.path(),
            "0.1.0",
        )
        .await
        .unwrap();
        assert!(session::current_session_path(dir.path()).is_file());
        tel.enqueue(TelemetryEvent::SessionMeta(events::SessionMetaEvent {
            ts_ms: 0,
            seq: 0,
            kind: events::SessionMetaKind::Started,
            fields: serde_json::Map::new(),
        }))
        .await
        .unwrap();
        let flushed = tel.flush(&http).await.unwrap();
        assert_eq!(flushed, 1);
    }
}

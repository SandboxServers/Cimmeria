//! Dev-session telemetry pipeline.

pub mod auth;
pub mod bundle;
pub mod chunk;
pub mod endpoint;
pub mod events;
mod flush;
pub mod install_result;
pub mod patch_counts;
pub mod patch_log;
pub mod process_watch;
pub mod queue;
pub mod runner;
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
use flush::{flush_queue, FlushTarget};
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
    #[error(transparent)]
    Endpoint(#[from] endpoint::EndpointError),
}

/// Live telemetry session. Holds the auth token, the disk queue for
/// overflow, and the running monotonic `seq` counter handed to each
/// event. Constructed by [`Telemetry::start_session`] after a
/// successful auth handshake; dropped when the game exits and the
/// bundle has been flushed.
pub struct Telemetry {
    /// Live token + endpoint + chunking defaults. Mutated by
    /// [`Telemetry::refresh_if_due`] when the proactive-refresh
    /// policy fires.
    pub session: tokio::sync::RwLock<DevSessionResponse>,
    pub auth_base_url: String,
    pub install_dir: PathBuf,
    pub install_id: String,
    pub machine_id: String,
    pub launcher_version: String,
    pub session_started_at_ms: i64,
    /// Where uploads may go; re-applied to the endpoint a refresh hands
    /// back.
    endpoint_policy: endpoint::EndpointPolicy,
    /// ms-since-epoch when the current token was issued — feeds
    /// `should_refresh` so the policy works without remembering the
    /// TTL separately.
    issued_at_ms: std::sync::atomic::AtomicI64,
    queue: DiskQueue,
    seq: Arc<Mutex<u64>>,
    /// ms-since-epoch before which [`Telemetry::flush`] sends nothing: the
    /// `Retry-After` of the last 429 or 503.
    retry_not_before_ms: std::sync::atomic::AtomicI64,
}

/// How far before the session's start a log file's modification time may
/// be and still count as this session's in the bundle.
const SESSION_FILE_SLACK_MS: i64 = 10_000;

impl Telemetry {
    /// One-call session bootstrap: handshake with `auth_base_url`,
    /// write `current-session.json`, return a live `Telemetry`
    /// handle. Callers feed events through [`Telemetry::enqueue`] and
    /// invoke [`Telemetry::flush`] on the flush cadence.
    ///
    /// `endpoint_policy` decides which plain-http addresses are allowed
    /// (see [`endpoint`]); build it from the launcher's login servers.
    pub async fn start_session(
        http: &reqwest::Client,
        auth_base_url: &str,
        req: DevSessionRequest,
        install_dir: &std::path::Path,
        launcher_version: &str,
        endpoint_policy: endpoint::EndpointPolicy,
    ) -> Result<Self, TelemetryError> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        // Refuse before sending anything, and refuse an upload endpoint
        // the server hands back that the same rule would refuse.
        endpoint_policy.check(auth_base_url)?;
        let resp = auth::fetch_dev_session(http, auth_base_url, &req).await?;
        endpoint_policy.check(&resp.upload_endpoint)?;
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
            session: tokio::sync::RwLock::new(resp),
            auth_base_url: auth_base_url.to_string(),
            install_dir: install_dir.to_path_buf(),
            install_id: req.install_id,
            machine_id: req.machine_id,
            launcher_version: launcher_version.to_string(),
            session_started_at_ms: now_ms,
            endpoint_policy,
            issued_at_ms: std::sync::atomic::AtomicI64::new(now_ms),
            queue: DiskQueue::new(&exe_dir()),
            seq: Arc::new(Mutex::new(0)),
            retry_not_before_ms: std::sync::atomic::AtomicI64::new(0),
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

    /// Drain the queue and POST it as chunks of at most `chunk_max_bytes`
    /// of NDJSON and [`chunk::MAX_CHUNK_ROWS`] rows, in order. Returns the
    /// events the server took.
    ///
    /// - A 413, or any 4xx but 401, 408 and 429, means the server will
    ///   never take that chunk: its events are dropped and added to the
    ///   queue's dropped-lines count (the bundle metadata reports it), and
    ///   the next chunk is sent.
    /// - Any other failure re-enqueues that chunk and every later one, so
    ///   the next flush replays them, and propagates the error (a
    ///   `TokenRejected` lets the caller refresh and retry).
    /// - A 429 or 503 also holds off further flushes for its
    ///   `Retry-After`; the queue stays on disk meanwhile.
    pub async fn flush(&self, http: &reqwest::Client) -> Result<u64, TelemetryError> {
        let (endpoint, token, max_bytes) = {
            let s = self.session.read().await;
            (
                s.upload_endpoint.clone(),
                s.token.clone(),
                s.chunk_max_bytes,
            )
        };
        let target = FlushTarget {
            endpoint: &endpoint,
            token: &token,
            max_bytes,
        };
        flush_queue(http, &self.queue, &target, &self.retry_not_before_ms).await
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
        let (endpoint, token, session_id) = {
            let s = self.session.read().await;
            (
                s.upload_endpoint.clone(),
                s.token.clone(),
                s.session_id.clone(),
            )
        };
        let metadata = bundle::BundleMetadata {
            session_id,
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
        // A little slack before the start: file times on some volumes
        // have a 2 s granularity, and the client may open its log as the
        // session is minted.
        let since = std::time::UNIX_EPOCH
            + std::time::Duration::from_millis(
                u64::try_from(
                    self.session_started_at_ms
                        .saturating_sub(SESSION_FILE_SLACK_MS),
                )
                .unwrap_or(0),
            );
        Ok(
            bundle::upload_bundle(http, &endpoint, &token, &self.install_dir, since, metadata)
                .await?,
        )
    }

    /// Run the proactive-refresh policy. Returns `Ok(true)` if a
    /// refresh fired and the in-memory token was rotated; `Ok(false)`
    /// when the current token still has ample lifetime.
    pub async fn refresh_if_due(&self, http: &reqwest::Client) -> Result<bool, TelemetryError> {
        let (expires_at_ms, current_token) = {
            let s = self.session.read().await;
            (s.expires_at_ms, s.token.clone())
        };
        let now_ms = chrono::Utc::now().timestamp_millis();
        let issued_at_ms = self.issued_at_ms.load(std::sync::atomic::Ordering::Relaxed);
        if !auth::should_refresh(expires_at_ms, now_ms, issued_at_ms) {
            return Ok(false);
        }
        let new_resp = auth::refresh_dev_session(http, &self.auth_base_url, &current_token).await?;
        self.endpoint_policy.check(&new_resp.upload_endpoint)?;
        self.issued_at_ms
            .store(now_ms, std::sync::atomic::Ordering::Relaxed);
        *self.session.write().await = new_resp;
        Ok(true)
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
        TelemetryEvent::ClientNative(e) => {
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

    /// Fresh-install defaults work together: the default auth URL is on a
    /// default login server. It lives here, not in `endpoint.rs`, because the
    /// desktop launcher's engine compiles that file too and has no `config`.
    #[test]
    fn the_default_auth_url_passes_with_the_default_login_servers() {
        let servers = crate::client_setup::login_servers::default_servers();
        let policy =
            endpoint::EndpointPolicy::from_login_servers(servers.iter().map(|s| s.url.as_str()));
        let auth_url = crate::config::TelemetrySettings::default().auth_url;
        assert_eq!(policy.check(&auth_url), Ok(()));
        // Without the login servers the same URL is refused: the login
        // server list is what vouches for it.
        assert!(endpoint::EndpointPolicy::default()
            .check(&auth_url)
            .is_err());
    }

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
            TelemetryEvent::ClientNative(events::ClientNativeEvent {
                ts_ms: 0,
                seq: 0,
                target: "client.frame_tick".into(),
                level: "info".into(),
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
                TelemetryEvent::ClientNative(e) => e.seq,
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
        // The client the worker uses. The launcher's shared https-only
        // client failed exactly here against the default
        // http://localhost auth URL.
        let http = endpoint::client();
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
            endpoint::EndpointPolicy::default(),
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

    fn request() -> DevSessionRequest {
        DevSessionRequest {
            install_id: "i".into(),
            machine_id: "m".into(),
            branch: "main".into(),
            git_sha: "abc".into(),
            launcher_version: "0.1.0".into(),
            tags: vec![],
        }
    }

    /// Plain http to another machine is refused before anything is sent
    /// (the host does not resolve, so a request would fail differently).
    #[tokio::test]
    async fn start_session_refuses_plain_http_to_another_machine() {
        let dir = tempfile::tempdir().unwrap();
        let err = Telemetry::start_session(
            &endpoint::client(),
            "http://telemetry.invalid:8443/api",
            request(),
            dir.path(),
            "0.1.0",
            endpoint::EndpointPolicy::default(),
        )
        .await
        .err()
        .expect("must refuse");
        assert!(
            matches!(
                err,
                TelemetryError::Endpoint(endpoint::EndpointError::InsecureRemote(_))
            ),
            "{err}"
        );
        assert!(!session::current_session_path(dir.path()).exists());
    }

    /// A server that hands back an unencrypted remote upload endpoint
    /// gets no session: nothing would be uploaded there.
    #[tokio::test]
    async fn start_session_refuses_an_insecure_upload_endpoint() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sid-1",
                "token": "tok",
                "expires_at_ms": 1_700_028_800_000_i64,
                "upload_endpoint": "http://uploads.example.org/api",
                "chunk_max_bytes": 1_048_576,
                "flush_interval_ms": 2_000,
            })))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let err = Telemetry::start_session(
            &endpoint::client(),
            &server.uri(),
            request(),
            dir.path(),
            "0.1.0",
            endpoint::EndpointPolicy::default(),
        )
        .await
        .err()
        .expect("must refuse");
        assert!(matches!(err, TelemetryError::Endpoint(_)), "{err}");
        assert!(!session::current_session_path(dir.path()).exists());
    }
}

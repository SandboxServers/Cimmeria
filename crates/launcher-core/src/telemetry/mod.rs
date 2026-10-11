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

use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::Mutex;

use auth::{DevSessionRequest, DevSessionResponse};
use chunk::ChunkError;
use events::TelemetryEvent;
pub use flush::MAX_CHUNKS_PER_FLUSH;
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
    /// One flush at a time: two would send the same in-flight chunk.
    flush_turn: Mutex<()>,
}

/// How far before the session's start a log file's modification time may
/// be and still count as this session's in the bundle.
const SESSION_FILE_SLACK_MS: i64 = 10_000;

/// Everything a [`Telemetry`] holds besides the minted token, for a
/// caller that minted the session itself ([`Telemetry::from_minted`]).
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// The base the mint went to; refreshes go to `<base>/auth/dev-session/refresh`.
    pub auth_base_url: String,
    /// The game installation whose session files the bundle carries.
    pub install_dir: PathBuf,
    pub install_id: String,
    pub machine_id: String,
    pub launcher_version: String,
    /// When the session was minted, ms since the epoch.
    pub session_started_at_ms: i64,
    pub endpoint_policy: endpoint::EndpointPolicy,
    /// The directory holding `telemetry-queue.jsonl`.
    pub queue_dir: PathBuf,
}

impl Telemetry {
    /// A live handle for a session the caller minted (the desktop launcher
    /// mints with its own bounded client and writes the DLL's marker
    /// itself). Nothing is sent and nothing is written; the upload endpoint
    /// is checked against `ctx.endpoint_policy` like a mint's.
    pub fn from_minted(
        resp: DevSessionResponse,
        ctx: SessionContext,
    ) -> Result<Self, TelemetryError> {
        ctx.endpoint_policy.check(&resp.upload_endpoint)?;
        Ok(Self {
            session: tokio::sync::RwLock::new(resp),
            auth_base_url: ctx.auth_base_url,
            install_dir: ctx.install_dir,
            install_id: ctx.install_id,
            machine_id: ctx.machine_id,
            launcher_version: ctx.launcher_version,
            session_started_at_ms: ctx.session_started_at_ms,
            endpoint_policy: ctx.endpoint_policy,
            issued_at_ms: std::sync::atomic::AtomicI64::new(ctx.session_started_at_ms),
            queue: DiskQueue::new(&ctx.queue_dir),
            seq: Arc::new(Mutex::new(0)),
            retry_not_before_ms: std::sync::atomic::AtomicI64::new(0),
            flush_turn: Mutex::new(()),
        })
    }

    /// One-call session bootstrap: handshake with `auth_base_url`,
    /// write `current-session.json`, return a live `Telemetry`
    /// handle. Callers feed events through [`Telemetry::enqueue`] and
    /// invoke [`Telemetry::flush`] on the flush cadence.
    ///
    /// `endpoint_policy` decides which plain-http addresses are allowed
    /// (see [`endpoint`]); build it from the launcher's login servers.
    /// `queue_dir` holds the on-disk overflow queue: the egui launcher
    /// passes its own directory, which is where [`recover_pending_on_startup`]
    /// looks on the next start.
    pub async fn start_session(
        http: &reqwest::Client,
        auth_base_url: &str,
        req: DevSessionRequest,
        install_dir: &Path,
        launcher_version: &str,
        endpoint_policy: endpoint::EndpointPolicy,
        queue_dir: &Path,
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
        Self::from_minted(
            resp,
            SessionContext {
                auth_base_url: auth_base_url.to_string(),
                install_dir: install_dir.to_path_buf(),
                install_id: req.install_id,
                machine_id: req.machine_id,
                launcher_version: launcher_version.to_string(),
                session_started_at_ms: now_ms,
                endpoint_policy,
                queue_dir: queue_dir.to_path_buf(),
            },
        )
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
    /// - Any other failure leaves that chunk and every later one on disk,
    ///   so the next flush sends them, and propagates the error (a
    ///   `TokenRejected` lets the caller refresh and retry).
    /// - A 429 or 503 also holds off further flushes for its
    ///   `Retry-After` (an hour at most); the queue stays on disk meanwhile.
    /// - A chunk leaves the disk only once the server has answered for it,
    ///   so dropping this future (the launcher closing) loses nothing.
    /// - One call sends at most `max_chunks` chunks:
    ///   [`MAX_CHUNKS_PER_FLUSH`] on the flush cadence, `usize::MAX` for the
    ///   final flush at game exit. Enqueues are not held up while a chunk
    ///   is on the wire; overlapping flushes of one handle take turns.
    pub async fn flush(
        &self,
        http: &reqwest::Client,
        max_chunks: usize,
    ) -> Result<u64, TelemetryError> {
        let _turn = self.flush_turn.lock().await;
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
        flush_queue(
            http,
            &self.queue,
            &target,
            &self.retry_not_before_ms,
            max_chunks,
        )
        .await
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
        let expires_at_ms = self.session.read().await.expires_at_ms;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let issued_at_ms = self.issued_at_ms.load(std::sync::atomic::Ordering::Relaxed);
        if !auth::should_refresh(expires_at_ms, now_ms, issued_at_ms) {
            return Ok(false);
        }
        self.refresh_now(http).await?;
        Ok(true)
    }

    /// Refresh the token whatever its age: what a 401 from an upload
    /// calls for. The server refuses an expired token (401) and a chain
    /// past its session cap; the caller then mints a new session and
    /// hands it to [`Telemetry::replace_session`].
    pub async fn refresh_now(&self, http: &reqwest::Client) -> Result<(), TelemetryError> {
        let current_token = self.session.read().await.token.clone();
        let new_resp = auth::refresh_dev_session(http, &self.auth_base_url, &current_token).await?;
        self.replace_session(new_resp).await
    }

    /// Swap in a newly minted or refreshed session: the token, endpoint and
    /// chunking defaults. The upload endpoint passes the session's policy
    /// first; the queue, `seq` counter and back-off clock carry over.
    pub async fn replace_session(&self, resp: DevSessionResponse) -> Result<(), TelemetryError> {
        self.endpoint_policy.check(&resp.upload_endpoint)?;
        self.issued_at_ms.store(
            chrono::Utc::now().timestamp_millis(),
            std::sync::atomic::Ordering::Relaxed,
        );
        *self.session.write().await = resp;
        Ok(())
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

/// Count the telemetry events a previous launcher run left on disk (it
/// crashed, was killed, or closed mid-flush); the next flush sends them.
/// Nothing is moved or rewritten, so a crash here loses nothing either;
/// only a line torn by a crash mid-append is ended, so the next append
/// does not join it. `queue_dir` is the directory
/// [`Telemetry::start_session`] was given. The recovered events keep their
/// old `seq` and go out under the next session's token, so the server
/// attributes them to that session and counts them against its limits.
pub fn recover_pending_on_startup(queue_dir: &Path) -> u64 {
    let q = DiskQueue::new(queue_dir);
    recover_pending_at(&q)
}

fn recover_pending_at(q: &DiskQueue) -> u64 {
    if let Err(e) = q.repair_torn_tail() {
        tracing::warn!(error = %e, "telemetry queue tail not repaired");
    }
    match q.pending_count::<TelemetryEvent>() {
        Ok(0) => 0,
        Ok(n) => {
            tracing::info!(
                pending = n,
                dropped_since_last_drain = q.dropped_count(),
                "Recovered telemetry events from previous session — will replay on next flush"
            );
            n
        }
        Err(e) => {
            tracing::warn!(error = %e, "telemetry recovery count failed");
            0
        }
    }
}

#[cfg(test)]
mod recovery_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::events::TelemetryEvent;

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
            dir.path(),
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
        let flushed = tel.flush(&http, MAX_CHUNKS_PER_FLUSH).await.unwrap();
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
            dir.path(),
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
            dir.path(),
        )
        .await
        .err()
        .expect("must refuse");
        assert!(matches!(err, TelemetryError::Endpoint(_)), "{err}");
        assert!(!session::current_session_path(dir.path()).exists());
    }

    fn minted(upload_endpoint: &str) -> DevSessionResponse {
        DevSessionResponse {
            session_id: "sid-1".into(),
            token: "tok".into(),
            expires_at_ms: 1_700_028_800_000,
            upload_endpoint: upload_endpoint.into(),
            chunk_max_bytes: 1_048_576,
            flush_interval_ms: 2_000,
        }
    }

    fn context(dir: &Path) -> SessionContext {
        SessionContext {
            auth_base_url: "http://localhost:8081/api".into(),
            install_dir: dir.to_path_buf(),
            install_id: "i".into(),
            machine_id: "m".into(),
            launcher_version: "0.1.0".into(),
            session_started_at_ms: 1_700_000_000_000,
            endpoint_policy: endpoint::EndpointPolicy::default(),
            queue_dir: dir.to_path_buf(),
        }
    }

    /// A session minted elsewhere is held to the same address rule, both
    /// when adopted and when a later refresh or re-mint replaces it, and
    /// adopting it writes no marker.
    #[tokio::test]
    async fn an_adopted_session_is_held_to_the_endpoint_policy() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            Telemetry::from_minted(
                minted("http://uploads.example.org/api"),
                context(dir.path())
            ),
            Err(TelemetryError::Endpoint(_))
        ));
        let tel = Telemetry::from_minted(minted("http://localhost:8081/api"), context(dir.path()))
            .unwrap();
        assert!(!session::current_session_path(dir.path()).exists());
        assert!(matches!(
            tel.replace_session(minted("http://uploads.example.org/api"))
                .await,
            Err(TelemetryError::Endpoint(_))
        ));
        assert_eq!(
            tel.session.read().await.upload_endpoint,
            "http://localhost:8081/api"
        );
        let mut next = minted("https://uploads.example.org/api");
        next.token = "tok-2".into();
        tel.replace_session(next).await.unwrap();
        assert_eq!(tel.session.read().await.token, "tok-2");
    }
}

//! Game-telemetry transport: the launcher's own uploads for an opted-in
//! player's session, on top of `cimmeria_launcher_core::telemetry` (the
//! queue, chunking, back-off and refresh code the Windows launcher uses).
//!
//! - **Queue.** Events go to the drop-oldest disk queue
//!   `telemetry-queue.jsonl` in the launcher state directory (100 MiB cap,
//!   dropped lines counted in `telemetry-queue.dropped`). It survives a
//!   launcher restart and is replayed by the next session's first flush.
//! - **Upload.** Gzipped NDJSON chunks of at most the mint's
//!   `chunk_max_bytes` and 1,000 rows, POSTed to `<upload_endpoint>/upload-chunk`.
//!   A 429 or 503 holds further flushes off for its `Retry-After` (60 s if
//!   absent); the events stay queued. A 413 or other permanent 4xx drops
//!   that chunk into the dropped-lines count.
//! - **Token.** Refreshed when 75% of its life has gone, and at once on a
//!   401, then the flush is retried once. A refresh the server refuses (an
//!   expired token, or a chain past its session cap) mints a new session
//!   with the same identity, bounded like Play's mint (4 s connect, 8 s
//!   total).
//! - **Addresses.** https anywhere; plain http only to this machine or the
//!   host and port of a login server
//!   ([`EndpointPolicy`](crate::telemetry_endpoint::EndpointPolicy)),
//!   for the mint, every refresh and every upload endpoint handed back.
//! - **Consent.** Nothing here runs without [`Consent`], which exists only
//!   while the player has opted in to game telemetry. At startup
//!   [`recover_on_startup`] keeps an opted-in player's queue and deletes an
//!   opted-out player's, so withdrawn data is never sent after a later
//!   opt-in.
//!
//! API for the session and event packets (LX-08b, LX-08c):
//!
//! | Call | Use |
//! |---|---|
//! | [`recover_on_startup`]`(&state).await` | once, when the launcher opens its state directory |
//! | [`Consent::read`]`(&state)` | under the state lock; `None` means opted out, do nothing |
//! | [`queue_without_session`]`(&consent, event)` | an event with no game session yet (install result); ships with the next session |
//! | [`Transport::open`]`(consent, &session, &login_servers, install_dir)` | after Play minted `session` |
//! | [`Transport::enqueue`]`(event)` | stamp `seq` (and `ts_ms` if 0) and queue |
//! | [`Transport::flush`]`()` | on the [`Transport::flush_interval`] cadence; returns events sent |
//! | [`Transport::upload_bundle`]`(event_count)` | at game exit, the end-of-session log bundle |
//! | [`Transport::shutdown`]`()` | final flush; whatever is left stays queued on disk |
//!
//! Every call that touches the queue takes one process-wide lock, so an
//! enqueue can never land between a flush's drain and its re-queue. Errors
//! are [`TransportError`] codes: no token, key, URL or server text.
mod queue;
#[cfg(test)]
mod tests;

pub use cimmeria_launcher_core::telemetry::bundle::BundleOutcome;
pub use cimmeria_launcher_core::telemetry::events::TelemetryEvent;
pub use queue::{queue_without_session, recover_on_startup, Consent, Recovery};

use super::{session, Identity, Session};
use crate::client_setup::login_servers::LoginServer;
use crate::telemetry_endpoint::EndpointPolicy;
use cimmeria_launcher_core::telemetry::auth::{AuthError, DevSessionResponse};
use cimmeria_launcher_core::telemetry::bundle::BundleError;
use cimmeria_launcher_core::telemetry::chunk::ChunkError;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use cimmeria_launcher_core::telemetry::{SessionContext, Telemetry, TelemetryError};
use queue::QUEUE_LOCK;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A chunk is at most a megabyte of NDJSON before gzip, and the bundle a
/// zip of one session's logs: far more than a mint's 8 s on a slow line.
const UPLOAD_TOTAL: Duration = Duration::from_secs(60);

/// What went wrong, as a code safe for logs and IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum TransportError {
    #[error("game telemetry is off")]
    NotOptedIn,
    #[error("the game-telemetry choice could not be read")]
    Choice,
    #[error("the server gave no usable telemetry session")]
    SessionUnavailable,
    #[error("a telemetry address is plain http to a host that is not a login server")]
    EndpointRefused,
    #[error("the telemetry queue could not be read or written")]
    Queue,
    #[error("the server refused the telemetry token")]
    TokenRejected,
    #[error("the server asked the launcher to wait before sending more")]
    BackingOff,
    #[error("the telemetry upload failed")]
    Upload,
}

impl TransportError {
    fn of(error: &TelemetryError) -> Self {
        match error {
            TelemetryError::Endpoint(_) => Self::EndpointRefused,
            TelemetryError::Queue(_) | TelemetryError::Session(_) => Self::Queue,
            TelemetryError::Auth(AuthError::Status { status: 401, .. })
            | TelemetryError::Chunk(ChunkError::TokenRejected)
            | TelemetryError::Bundle(BundleError::TokenRejected) => Self::TokenRejected,
            TelemetryError::Auth(AuthError::Status { status: 429, .. })
            | TelemetryError::Auth(AuthError::KillSwitch { .. })
            | TelemetryError::Bundle(BundleError::KillSwitch { .. }) => Self::BackingOff,
            TelemetryError::Chunk(e) if e.retry_after_secs().is_some() => Self::BackingOff,
            TelemetryError::Auth(_) => Self::SessionUnavailable,
            TelemetryError::Chunk(_) | TelemetryError::Bundle(_) => Self::Upload,
        }
    }
}

/// One game session's uploads. Built from the session Play minted, so the
/// DLL and the launcher start on the same token.
pub struct Transport {
    telemetry: Telemetry,
    /// Refresh and re-mint: Play's 8 s bound.
    auth: reqwest::Client,
    /// Chunks and the bundle.
    upload: reqwest::Client,
    identity: Identity,
    login_servers: Vec<LoginServer>,
    tags: Vec<String>,
    queue_dir: PathBuf,
}

impl Transport {
    /// Adopt `session` (minted against `login_servers.first()`). Sends
    /// nothing. `install_dir` is the game directory whose
    /// `Binaries/sgwdebuglog*` and `Binaries/sessions/` the bundle carries.
    pub fn open(
        consent: Consent,
        session: &Session,
        login_servers: &[LoginServer],
        install_dir: &Path,
    ) -> Result<Self, TransportError> {
        let server = login_servers
            .first()
            .ok_or(TransportError::SessionUnavailable)?;
        let marker = session.marker();
        let context = SessionContext {
            auth_base_url: session::auth_base_url(&server.url),
            install_dir: install_dir.to_path_buf(),
            install_id: marker.install_id.clone(),
            machine_id: marker.machine_id.clone(),
            launcher_version: session::LAUNCHER_VERSION.into(),
            session_started_at_ms: marker.session_started_at_ms,
            endpoint_policy: EndpointPolicy::from_login_servers(
                login_servers.iter().map(|s| s.url.as_str()),
            ),
            queue_dir: consent.queue_dir.clone(),
        };
        let telemetry =
            Telemetry::from_minted(minted(session), context).map_err(|e| TransportError::of(&e))?;
        Ok(Self {
            telemetry,
            auth: client(session::TOTAL)?,
            upload: client(UPLOAD_TOTAL)?,
            identity: consent.identity,
            login_servers: login_servers.to_vec(),
            tags: marker.tags.clone(),
            queue_dir: consent.queue_dir,
        })
    }

    /// The flush cadence the server asked for, at least 250 ms.
    pub async fn flush_interval(&self) -> Duration {
        let ms = self.telemetry.session.read().await.flush_interval_ms;
        Duration::from_millis(ms.max(250))
    }

    /// Stamp the next `seq` (and `ts_ms` when 0) and append to the queue.
    pub async fn enqueue(&self, event: TelemetryEvent) -> Result<(), TransportError> {
        let _queue = QUEUE_LOCK.lock().await;
        self.telemetry
            .enqueue(event)
            .await
            .map_err(|e| TransportError::of(&e))
    }

    /// Send what is queued. Returns the events the server took; `Ok(0)`
    /// during a back-off. On an error the unsent events stay queued.
    pub async fn flush(&self) -> Result<u64, TransportError> {
        let _queue = QUEUE_LOCK.lock().await;
        self.refresh_if_due().await;
        let result = match self.telemetry.flush(&self.upload).await {
            Err(TelemetryError::Chunk(ChunkError::TokenRejected)) => {
                tracing::info!("telemetry token refused by the upload endpoint; renewing");
                self.renew().await?;
                self.telemetry.flush(&self.upload).await
            }
            other => other,
        };
        result.map_err(|e| {
            let code = TransportError::of(&e);
            tracing::warn!(?code, "telemetry flush failed; events stay queued");
            code
        })
    }

    /// Zip this session's client logs and post them with the session's
    /// totals. `Ok(None)`: no log was written this session, nothing sent.
    /// After a bundle is taken the dropped-lines counter starts again.
    pub async fn upload_bundle(
        &self,
        event_count: u64,
    ) -> Result<Option<BundleOutcome>, TransportError> {
        let _queue = QUEUE_LOCK.lock().await;
        let queue = DiskQueue::new(&self.queue_dir);
        let dropped = queue.dropped_count();
        let mut result = self
            .telemetry
            .upload_bundle(&self.upload, event_count, dropped)
            .await;
        if matches!(
            result,
            Err(TelemetryError::Bundle(BundleError::TokenRejected))
        ) {
            self.renew().await?;
            result = self
                .telemetry
                .upload_bundle(&self.upload, event_count, dropped)
                .await;
        }
        match result {
            Ok(outcome) => {
                if queue.dropped_counter_reset().is_err() {
                    tracing::warn!("telemetry dropped-lines counter not reset");
                }
                Ok(Some(outcome))
            }
            Err(TelemetryError::Bundle(BundleError::Empty)) => Ok(None),
            Err(e) => Err(TransportError::of(&e)),
        }
    }

    /// The last flush. Anything it cannot send stays queued for the next
    /// session.
    pub async fn shutdown(self) -> Result<u64, TransportError> {
        self.flush().await
    }

    /// The proactive refresh. A failure leaves the current token in use; a
    /// 401 from the upload will renew it.
    async fn refresh_if_due(&self) {
        if let Err(e) = self.telemetry.refresh_if_due(&self.auth).await {
            let code = TransportError::of(&e);
            if code == TransportError::TokenRejected {
                if let Err(code) = self.remint().await {
                    tracing::warn!(?code, "telemetry re-mint failed");
                }
            } else {
                tracing::warn!(?code, "proactive telemetry token refresh failed");
            }
        }
    }

    /// After a 401: refresh, or mint a new session when the refresh is
    /// refused too.
    async fn renew(&self) -> Result<(), TransportError> {
        match self.telemetry.refresh_now(&self.auth).await {
            Ok(()) => Ok(()),
            Err(e) => match TransportError::of(&e) {
                TransportError::TokenRejected => self.remint().await,
                code => Err(code),
            },
        }
    }

    async fn remint(&self) -> Result<(), TransportError> {
        let tags: Vec<&str> = self.tags.iter().map(String::as_str).collect();
        let session = super::start(&self.identity, &self.login_servers, &tags)
            .await
            .map_err(|outcome| match outcome {
                super::Outcome::EndpointRefused => TransportError::EndpointRefused,
                _ => TransportError::SessionUnavailable,
            })?;
        tracing::info!("telemetry session re-minted after its token was refused");
        self.telemetry
            .replace_session(minted(&session))
            .await
            .map_err(|e| TransportError::of(&e))
    }
}

/// The marker Play wrote, as the reply core's session holds.
fn minted(session: &Session) -> DevSessionResponse {
    let marker = session.marker();
    DevSessionResponse {
        session_id: marker.session_id.clone(),
        token: marker.telemetry.token.clone(),
        expires_at_ms: marker.telemetry.expires_at_ms,
        upload_endpoint: marker.telemetry.upload_endpoint.clone(),
        chunk_max_bytes: marker.telemetry.chunk_max_bytes,
        flush_interval_ms: marker.telemetry.flush_interval_ms,
    }
}

/// No redirects: a checked address must not bounce to one the policy refuses.
fn client(total: Duration) -> Result<reqwest::Client, TransportError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(session::CONNECT)
        .timeout(total)
        .build()
        .map_err(|_| TransportError::Upload)
}

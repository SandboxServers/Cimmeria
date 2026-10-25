//! Game-telemetry transport: the launcher's own uploads for an opted-in
//! player's session, on top of `cimmeria_launcher_core::telemetry` (the
//! queue, chunking, back-off and refresh code the Windows launcher uses).
//!
//! - **Queue.** Events go to the drop-oldest disk queue
//!   `telemetry-queue.jsonl` in the launcher state directory (100 MiB cap,
//!   dropped lines counted in `telemetry-queue.dropped`). It survives a
//!   launcher restart and is replayed by the next session's first flush,
//!   under that session's token: the events are not re-stamped, and the
//!   server attributes them to the new session and counts them against its
//!   upload limits. Delivery is at-least-once and the server does not
//!   dedupe, so a crash after a 2xx and before the queue records it sends
//!   that chunk twice.
//!   A flush moves it to `telemetry-queue.inflight.jsonl` and deletes each
//!   chunk only once the server has answered for it, so closing the
//!   launcher mid-flush loses nothing (at worst one chunk goes twice).
//! - **Upload.** Gzipped NDJSON chunks of at most the mint's
//!   `chunk_max_bytes` and 1,000 rows, POSTed to `<upload_endpoint>/upload-chunk`,
//!   at most eight per flush. A 429 or 503 holds further flushes off for
//!   its `Retry-After` (60 s if absent, an hour at most); the events stay
//!   queued. A 413 or other permanent 4xx drops that chunk into the
//!   dropped-lines count.
//! - **Token.** Refreshed when 75% of its life has gone, and at once on a
//!   401, then the flush is retried once. A refresh the server refuses (an
//!   expired token, or a chain past its session cap) mints a new session
//!   with the same identity, bounded like Play's mint (4 s connect, 8 s
//!   total). Replies past 16 KiB are refused unread. When renewing fails,
//!   or the refresh answers 429/503, or the upload refuses a token it was
//!   just renewed to, flushes hold off for 60 s, doubling on each further
//!   failure up to 30 minutes. Only an upload the server takes (a flush
//!   that sent events, a bundle) starts the doubling again; a renewal
//!   alone does not.
//! - **Addresses.** https anywhere; plain http only to this machine
//!   ([`EndpointPolicy`](crate::telemetry_endpoint::EndpointPolicy)), for the
//!   mint, every refresh and every upload endpoint handed back. An http login
//!   server on a DNS name is reached over https on 443.
//! - **Consent.** Nothing here runs without [`Consent`], which exists only
//!   while the player has opted in to game telemetry. Changing the choice
//!   (`DesktopState::set_game_telemetry`) deletes the queue unsent, and at
//!   startup [`recover_on_startup`] keeps an opted-in player's queue and
//!   deletes an opted-out player's, so withdrawn data is never sent after
//!   a later opt-in. A transport stops at the change too: every enqueue,
//!   flush and bundle checks its [`Consent`] is still current, before and
//!   after it writes, so an opt-out while the game runs leaves no queue
//!   behind (a chunk already being posted at that moment still completes).
//!
//! API for the session and event packets (LX-08b, LX-08c):
//!
//! | Call | Use |
//! |---|---|
//! | [`recover_on_startup`]`(&state).await`, or [`recover_now`]`(&state)` under the lock | once, when the launcher opens its state directory |
//! | [`Consent::read`]`(&state)` | under the state lock; `None` means opted out, do nothing |
//! | [`queue_without_session`]`(&consent, event)` | an event with no game session yet (install result); ships with the next session |
//! | [`Transport::open`]`(consent, &session, &login_servers, install_dir)` | after Play minted `session` |
//! | [`Transport::enqueue`]`(event)` | stamp `seq` (and `ts_ms` if 0) and queue |
//! | [`Transport::flush`]`()` | on the [`Transport::flush_interval`] cadence; returns events sent |
//! | [`Transport::upload_bundle`]`(event_count)` | at game exit, the end-of-session log bundle |
//! | [`Transport::shutdown`]`()` | final flush of the whole backlog; whatever is left stays queued on disk |
//!
//! Core's queue locks its files for each short operation, never across a
//! POST, so an enqueue never waits on a flush; flushes and bundle uploads
//! take turns across the process. Errors are [`TransportError`] codes: no
//! token, key, URL or server text.
pub(super) mod events;
#[cfg(test)]
mod events_tests;
mod queue;
#[cfg(test)]
mod tests;

pub use cimmeria_launcher_core::telemetry::bundle::BundleOutcome;
pub use cimmeria_launcher_core::telemetry::events::TelemetryEvent;
pub(crate) use queue::{discard_queue, queue_now};
pub use queue::{queue_without_session, recover_now, recover_on_startup, Consent, Recovery};

use super::{endpoint_sanity, session, Session};
use crate::client_setup::login_servers::LoginServer;
use crate::game_telemetry::ip_scrub::scrub_event;
use crate::telemetry_endpoint::EndpointPolicy;
use cimmeria_launcher_core::telemetry::auth::{AuthError, DevSessionResponse};
use cimmeria_launcher_core::telemetry::bundle::BundleError;
use cimmeria_launcher_core::telemetry::chunk::ChunkError;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use cimmeria_launcher_core::telemetry::{
    SessionContext, Telemetry, TelemetryError, MAX_CHUNKS_PER_FLUSH,
};
use queue::flush_turn;
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A chunk is at most a megabyte of NDJSON before gzip, and the bundle a
/// zip of one session's logs: far more than a mint's 8 s on a slow line.
const UPLOAD_TOTAL: Duration = Duration::from_secs(60);

/// After the server will not give the launcher a working token, flushes
/// wait this long, doubling on each further failure up to [`AUTH_HOLD_MAX`],
/// rather than refreshing and minting again on every flush.
const AUTH_HOLD_FIRST: Duration = Duration::from_secs(60);
const AUTH_HOLD_MAX: Duration = Duration::from_secs(30 * 60);

/// The token back-off: when flushes may run again, and the next wait.
#[derive(Debug)]
struct AuthHold {
    until: Option<Instant>,
    next: Duration,
}

impl AuthHold {
    fn new() -> Self {
        Self {
            until: None,
            next: AUTH_HOLD_FIRST,
        }
    }
}

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
    #[error("a telemetry address is plain http to another machine, so nothing was sent")]
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
    /// Checked around every queue write and send: an opt-out stops this
    /// transport mid-session.
    consent: Consent,
    login_servers: Vec<LoginServer>,
    tags: Vec<String>,
    auth_hold: Mutex<AuthHold>,
    streaks: events::Streaks,
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
            launcher_version: session::mint_launcher_version(),
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
            consent,
            login_servers: login_servers.to_vec(),
            tags: marker.tags.clone(),
            auth_hold: Mutex::new(AuthHold::new()),
            streaks: events::Streaks::default(),
        })
    }

    /// The flush cadence the server asked for, at least 250 ms.
    pub async fn flush_interval(&self) -> Duration {
        let ms = self.telemetry.session.read().await.flush_interval_ms;
        Duration::from_millis(ms.max(250))
    }

    /// Stamp the next `seq` (and `ts_ms` when 0) and append to the queue.
    /// Never waits on a flush in progress.
    /// After an opt-out: Err(NotOptedIn), and nothing is written.
    pub async fn enqueue(&self, mut event: TelemetryEvent) -> Result<(), TransportError> {
        if !self.consent.current() {
            return Err(TransportError::NotOptedIn);
        }
        // LT-40: no IP literal reaches the queue, whichever builder made the row.
        scrub_event(&mut event);
        let result = self
            .telemetry
            .enqueue(event)
            .await
            .map_err(|e| TransportError::of(&e));
        self.consent.undo_if_withdrawn()?;
        result
    }

    /// False once the player has changed their game-telemetry choice since
    /// this transport was opened: every call after that refuses. A chunk
    /// already being posted when the choice changed still completes.
    pub fn consented(&self) -> bool {
        self.consent.current()
    }

    /// The queue (and state) directory this transport writes to.
    pub(crate) fn queue_dir(&self) -> &Path {
        &self.consent.queue_dir
    }

    /// Send what is queued, at most eight chunks. Returns the events the
    /// server took; `Ok(0)` during a back-off. On an error, or if the future
    /// is dropped part-way (the launcher closing), the unsent events stay
    /// queued on disk.
    pub async fn flush(&self) -> Result<u64, TransportError> {
        self.flush_chunks(MAX_CHUNKS_PER_FLUSH).await
    }

    async fn flush_chunks(&self, max_chunks: usize) -> Result<u64, TransportError> {
        let turn = flush_turn(&self.consent.queue_dir);
        let _turn = turn.lock().await;
        if !self.consent.current() {
            return Err(TransportError::NotOptedIn);
        }
        let result = self.flush_in_turn(max_chunks).await;
        // A flush puts back what the server did not take; after an opt-out
        // that must not survive.
        self.consent.undo_if_withdrawn()?;
        result
    }

    async fn flush_in_turn(&self, max_chunks: usize) -> Result<u64, TransportError> {
        if self.auth_held() {
            return Ok(0);
        }
        self.refresh_if_due().await?;
        let result = match self.telemetry.flush(&self.upload, max_chunks).await {
            Err(TelemetryError::Chunk(ChunkError::TokenRejected)) => {
                events::token_renew_started();
                self.renew().await?;
                let retried = self.telemetry.flush(&self.upload, max_chunks).await;
                if matches!(
                    retried,
                    Err(TelemetryError::Chunk(ChunkError::TokenRejected))
                ) {
                    // Renewing again on every tick would only hammer the
                    // auth routes.
                    self.hold_after_refused_renewal();
                }
                retried
            }
            other => other,
        };
        match result {
            Ok(sent) => {
                // Events the server took prove the token works: the next
                // failure starts the back-off from its first step. (An
                // `Ok(0)` proves nothing; the queue may just be empty.)
                if sent > 0 {
                    self.release_auth_hold();
                    events::chunk_upload_resumed(&self.streaks.upload);
                }
                Ok(sent)
            }
            Err(e) => {
                events::chunk_upload_failed(&self.streaks.upload, &e);
                Err(TransportError::of(&e))
            }
        }
    }

    /// Zip this session's client logs and post them with the session's
    /// totals. `Ok(None)`: no log was written this session, nothing sent.
    /// After a bundle is taken the dropped-lines counter starts again.
    pub async fn upload_bundle(
        &self,
        event_count: u64,
    ) -> Result<Option<BundleOutcome>, TransportError> {
        let turn = flush_turn(&self.consent.queue_dir);
        let _turn = turn.lock().await;
        if !self.consent.current() {
            return Err(TransportError::NotOptedIn);
        }
        let queue = DiskQueue::new(&self.consent.queue_dir);
        let dropped = queue.dropped_count();
        let mut result = self
            .telemetry
            .upload_bundle(&self.upload, event_count, dropped)
            .await;
        if matches!(
            result,
            Err(TelemetryError::Bundle(BundleError::TokenRejected))
        ) && !self.auth_held()
        {
            self.renew().await?;
            result = self
                .telemetry
                .upload_bundle(&self.upload, event_count, dropped)
                .await;
            if matches!(
                result,
                Err(TelemetryError::Bundle(BundleError::TokenRejected))
            ) {
                self.hold_after_refused_renewal();
            }
        }
        match result {
            Ok(outcome) => {
                self.release_auth_hold();
                if dropped > 0 {
                    events::queue_dropped(dropped);
                }
                if queue.dropped_counter_reset().is_err() {
                    events::dropped_counter_reset_failed();
                }
                Ok(Some(outcome))
            }
            Err(TelemetryError::Bundle(BundleError::Empty)) => Ok(None),
            Err(e) => {
                events::bundle_upload_failed(&e);
                Err(TransportError::of(&e))
            }
        }
    }

    /// The last flush: the whole backlog, not the cadence's eight chunks.
    /// Anything it cannot send stays queued for the next session. A caller
    /// closing the launcher may bound it with a timeout: dropping the
    /// future part-way loses nothing.
    pub async fn shutdown(self) -> Result<u64, TransportError> {
        self.flush_chunks(usize::MAX).await
    }

    /// The proactive refresh. A failure that only means the server could
    /// not be reached leaves the current token in use (a 401 from the
    /// upload will renew it). A refused token is re-minted; a failed
    /// re-mint, or a 429/503 from the refresh, starts the token back-off
    /// and ends this flush.
    async fn refresh_if_due(&self) -> Result<(), TransportError> {
        let (code, status) = match self.telemetry.refresh_if_due(&self.auth).await {
            Ok(refreshed) => {
                self.streaks.refresh.reset();
                let pinned = if refreshed {
                    self.pin_endpoint().await
                } else {
                    Ok(())
                };
                return pinned.inspect_err(|code| {
                    events::token_refresh_failed("refresh", *code, None);
                    self.hold_auth();
                });
            }
            Err(e) => (TransportError::of(&e), events::http_status(&e)),
        };
        match code {
            TransportError::TokenRejected => match self.remint().await {
                Ok(()) => Ok(()),
                Err(code) => {
                    events::token_refresh_failed("remint", code, None);
                    self.hold_auth();
                    Err(code)
                }
            },
            TransportError::BackingOff => {
                events::token_refresh_failed("refresh", code, status);
                self.hold_auth();
                Err(code)
            }
            code => {
                events::proactive_refresh_failed(&self.streaks.refresh, code, status);
                Ok(())
            }
        }
    }

    /// Keep the C10 upload address after a refresh: the refresh reply may
    /// advertise the loopback address again, so it is replaced as at mint.
    /// Logs nothing; the mint's `upload_endpoint_overridden` row stands.
    async fn pin_endpoint(&self) -> Result<(), TransportError> {
        let server = self
            .login_servers
            .first()
            .ok_or(TransportError::SessionUnavailable)?;
        let mut session = self.telemetry.session.write().await;
        let effective =
            endpoint_sanity::effective_upload_endpoint(&server.url, &session.upload_endpoint)
                .ok_or(TransportError::EndpointRefused)?;
        session.upload_endpoint = effective.url;
        Ok(())
    }

    /// After a 401: refresh, or mint a new session when the refresh is
    /// refused too. Any failure starts the token back-off. A success leaves
    /// the back-off's next step alone: only an upload the server takes
    /// proves the new token works (a server that renews tokens and then
    /// refuses them must still see the waits double).
    async fn renew(&self) -> Result<(), TransportError> {
        let mut status = None;
        let result = match self.telemetry.refresh_now(&self.auth).await {
            Ok(()) => self.pin_endpoint().await,
            Err(e) => match TransportError::of(&e) {
                TransportError::TokenRejected => self.remint().await,
                code => {
                    status = events::http_status(&e);
                    Err(code)
                }
            },
        };
        result.inspect_err(|code| {
            events::token_refresh_failed("renew", *code, status);
            self.hold_auth();
        })
    }

    /// The upload endpoint refused the token it was just renewed to.
    fn hold_after_refused_renewal(&self) {
        events::token_refresh_failed(
            "upload_after_renew",
            TransportError::TokenRejected,
            Some(401),
        );
        self.hold_auth();
    }

    fn auth_held(&self) -> bool {
        let hold = self.auth_hold.lock().unwrap_or_else(|e| e.into_inner());
        hold.until.is_some_and(|until| Instant::now() < until)
    }

    /// Hold flushes off for the next wait, and double it.
    fn hold_auth(&self) {
        let mut hold = self.auth_hold.lock().unwrap_or_else(|e| e.into_inner());
        hold.until = Some(Instant::now() + hold.next);
        hold.next = (hold.next * 2).min(AUTH_HOLD_MAX);
    }

    fn release_auth_hold(&self) {
        *self.auth_hold.lock().unwrap_or_else(|e| e.into_inner()) = AuthHold::new();
    }

    async fn remint(&self) -> Result<(), TransportError> {
        let tags: Vec<&str> = self.tags.iter().map(String::as_str).collect();
        let session = super::start(&self.consent.identity, &self.login_servers, &tags)
            .await
            .map_err(|outcome| match outcome {
                super::Outcome::EndpointRefused => TransportError::EndpointRefused,
                _ => TransportError::SessionUnavailable,
            })?;
        events::session_reminted();
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

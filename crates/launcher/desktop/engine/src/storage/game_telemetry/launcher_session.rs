//! The launcher's own adoption events, sent at once under a `launcher` session
//! (C11). Nothing is queued on disk and the upload queue is never touched: a
//! send that cannot post drops its events (README D-AU21).
use super::session::{self, MintFailure, Session};
use super::*;
use crate::client_setup::login_servers::{self, LoginServer};
use crate::telemetry_endpoint::EndpointPolicy;
use cimmeria_launcher_core::telemetry::chunk::{self, ChunkError};
use cimmeria_launcher_core::telemetry::events::TelemetryEvent;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// The `session_kind` a launcher session is minted with (server C12).
pub const LAUNCHER_SESSION_KIND: &str = "launcher";
/// First hold after a failed mint; it doubles per failure in a row, up to MAX_HOLD.
pub const MINT_HOLD: Duration = Duration::from_secs(60);
/// The shortest hold after a 429 or 503 (R16).
pub const MIN_RETRY_HOLD: Duration = Duration::from_secs(60);
/// The longest hold of any kind.
pub const MAX_HOLD: Duration = Duration::from_secs(3_600);
/// The mint `purpose` of the log-ingest session (server C-LF4).
pub const LOGS_PURPOSE: &str = "logs";
/// Rows per chunk; the server keeps at most this many (C14).
pub const LAUNCHER_MAX_ROWS: usize = 200;
/// Re-mint when the token has less than this left.
pub const REMINT_MARGIN_MS: i64 = 60_000;

const LAUNCHER_TAGS: [&str; 3] = ["desktop-launcher", std::env::consts::OS, "launcher"];

/// What a launcher send needs: the opted-in identity and the login servers to mint on.
#[derive(Debug, Clone)]
pub struct LauncherTarget {
    /// The opted-in install and machine identity the session is minted for.
    pub identity: Identity,
    /// The login servers to mint on, in preference order.
    pub servers: Vec<LoginServer>,
}

impl DesktopState {
    /// `Some` only while the player is opted in to game diagnostics and has an
    /// identity. `servers` is the saved login-server preference, or the defaults
    /// when it is empty, validated. Never mints, never sends.
    pub fn launcher_target(&self) -> Option<LauncherTarget> {
        let choice = self.game_telemetry().ok()?;
        let identity = choice.opted_in.then_some(choice.identity).flatten()?;
        self.target_for(identity)
    }

    /// The target for "Upload debug logs" (LF-15, D-LU1): the press itself is the
    /// consent for that one upload, so the diagnostics opt-in does not matter.
    /// An install that has an identity uses it. One that never opted in has none
    /// (it is minted only at opt-in), so the target carries a fresh one that is
    /// not saved: it lasts for this upload, and the per-install budget the
    /// server keeps cannot tie two such presses together (the per-IP ceiling
    /// still applies). `None` only when the login-server list is unusable.
    pub fn launcher_log_target(&self) -> Option<LauncherTarget> {
        let stored = self
            .game_telemetry()
            .ok()
            .and_then(|choice| choice.identity);
        self.target_for(stored.unwrap_or_else(Identity::mint))
    }

    fn target_for(&self, identity: Identity) -> Option<LauncherTarget> {
        let servers = if self.preferences.login_servers.is_empty() {
            login_servers::default_servers()
        } else {
            self.preferences.login_servers.clone()
        };
        login_servers::validate(&servers).ok()?;
        Some(LauncherTarget { identity, servers })
    }
}

/// What one `send` did. The player never sees it; `Held` and `Dropped` are normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome {
    /// The batch had no events; nothing was posted.
    Empty,
    /// Every chunk was accepted.
    Sent,
    /// A hold from an earlier failure is still running; nothing was posted.
    Held,
    /// No session could be minted; a hold now follows. `reason` is a C8 code.
    MintFailed { reason: &'static str },
    /// Some events were not posted and were discarded.
    Dropped,
}

/// A minted session and the upload URL its batches post to: the advertised
/// address, or the C10 replacement when the advertised one is loopback.
#[derive(Debug, Clone)]
struct Live {
    session: Session,
    upload_url: String,
}

/// What one log chunk post did (LF-14). The 429/503 hold is not taken here: the
/// caller ends the upload `Partial` and the next press starts clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchOutcome {
    /// The chunk was accepted (2xx).
    Posted,
    /// A 429 or 503; the wait is the server's `Retry-After`.
    Throttled { retry_after_secs: u64 },
    /// No session could be minted; `reason` is a C8 code, or "held" while an earlier failed mint's hold is still running.
    MintFailed { reason: &'static str },
    /// Anything else, including a second 401.
    Failed,
}

/// Sends launcher events over one HTTP client, keeping one session and one hold.
pub struct LauncherSender {
    /// `Some("logs")` for a log-ingest session; it never shares a token with
    /// the event session (the server picks limits by this claim).
    purpose: Option<&'static str>,
    http: reqwest::Client,
    session: Mutex<Option<Live>>,
    hold_until: Mutex<Option<Instant>>,
    mint_failures: Mutex<u32>,
}

impl Default for LauncherSender {
    fn default() -> Self {
        Self::new()
    }
}

impl LauncherSender {
    /// A sender with no session and no hold. Its client follows no redirect, so
    /// the bearer token never reaches another host.
    pub fn new() -> Self {
        Self::with_purpose(None)
    }

    /// A sender whose session is minted with `purpose = "logs"`.
    pub fn logs() -> Self {
        Self::with_purpose(Some(LOGS_PURPOSE))
    }

    fn with_purpose(purpose: Option<&'static str>) -> Self {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(session::CONNECT)
            .timeout(session::TOTAL)
            .build()
            .expect("the TLS backend is compiled in");
        Self {
            purpose,
            http,
            session: Mutex::new(None),
            hold_until: Mutex::new(None),
            mint_failures: Mutex::new(0),
        }
    }

    /// Send `events` now, in chunks. Never panics, never writes to disk, never
    /// logs a field value, never logs at target `adoption`.
    pub async fn send(&self, target: &LauncherTarget, events: &[TelemetryEvent]) -> SendOutcome {
        if events.is_empty() {
            return SendOutcome::Empty;
        }
        if self.held() {
            tracing::debug!(
                target: "launcher.pipeline",
                reason = "held",
                "launcher telemetry batch not sent"
            );
            return SendOutcome::Held;
        }
        let mut live = match self.usable_session(target).await {
            Ok(live) => live,
            Err(reason) => {
                tracing::debug!(
                    target: "launcher.pipeline",
                    reason,
                    "launcher telemetry batch not sent"
                );
                return SendOutcome::MintFailed { reason };
            }
        };
        let max_bytes = live.session.marker().telemetry.chunk_max_bytes;
        let Ok(batches) = chunk::split_batches(events, max_bytes, LAUNCHER_MAX_ROWS) else {
            tracing::debug!(
                target: "launcher.pipeline",
                reason = "dropped",
                "launcher telemetry batch not sent"
            );
            return SendOutcome::Dropped;
        };
        for range in batches {
            let batch = &events[range];
            let mut posted = self.post(&live, batch).await;
            if matches!(posted, Err(ChunkError::TokenRejected)) {
                // A 401 is the only error that mints again, and only once.
                *locked(&self.session) = None;
                let Ok(fresh) = self.mint(target).await else {
                    tracing::debug!(
                        target: "launcher.pipeline",
                        reason = "dropped",
                        "launcher telemetry batch not sent"
                    );
                    return SendOutcome::Dropped;
                };
                live = fresh;
                posted = self.post(&live, batch).await;
            }
            match posted {
                Ok(()) => {}
                Err(err @ (ChunkError::KillSwitch { .. } | ChunkError::RateLimited { .. })) => {
                    let asked = err.retry_after_secs().unwrap_or(60);
                    self.hold(Duration::from_secs(asked).clamp(MIN_RETRY_HOLD, MAX_HOLD));
                    tracing::debug!(
                        target: "launcher.pipeline",
                        reason = "dropped",
                        "launcher telemetry batch not sent"
                    );
                    return SendOutcome::Dropped;
                }
                Err(_) => {
                    tracing::debug!(
                        target: "launcher.pipeline",
                        reason = "dropped",
                        "launcher telemetry batch not sent"
                    );
                    return SendOutcome::Dropped;
                }
            }
        }
        SendOutcome::Sent
    }

    /// Post one already-split log chunk (LF-14). A 401 mints again, once.
    /// Unlike `send`, a throttle does not hold the sender: the caller paces and
    /// ends the upload itself. A hold from a failed mint does stop a mint while
    /// no session is stored, so a retry loop cannot use up the mint budget that
    /// the event session shares.
    pub async fn post_batch(
        &self,
        target: &LauncherTarget,
        batch: &[TelemetryEvent],
    ) -> BatchOutcome {
        if self.held() && locked(&self.session).is_none() {
            return BatchOutcome::MintFailed { reason: "held" };
        }
        let mut live = match self.usable_session(target).await {
            Ok(live) => live,
            Err(reason) => return BatchOutcome::MintFailed { reason },
        };
        let mut posted = self.post(&live, batch).await;
        if matches!(posted, Err(ChunkError::TokenRejected)) {
            *locked(&self.session) = None;
            live = match self.mint(target).await {
                Ok(fresh) => fresh,
                Err(reason) => return BatchOutcome::MintFailed { reason },
            };
            posted = self.post(&live, batch).await;
        }
        match posted {
            Ok(()) => BatchOutcome::Posted,
            Err(err @ (ChunkError::KillSwitch { .. } | ChunkError::RateLimited { .. })) => {
                BatchOutcome::Throttled {
                    retry_after_secs: err.retry_after_secs().unwrap_or(60),
                }
            }
            Err(_) => BatchOutcome::Failed,
        }
    }

    /// The stored session while it has time left, else a fresh mint.
    async fn usable_session(&self, target: &LauncherTarget) -> Result<Live, &'static str> {
        let stored = locked(&self.session).clone();
        match stored {
            Some(live) if !expiring(&live.session) => Ok(live),
            _ => self.mint(target).await,
        }
    }

    /// Mint a launcher session. A failure holds the sender, doubling per failure
    /// in a row up to `MAX_HOLD`.
    async fn mint(&self, target: &LauncherTarget) -> Result<Live, &'static str> {
        let minted = session::start_kind(
            &target.identity,
            &target.servers,
            &LAUNCHER_TAGS,
            Some(LAUNCHER_SESSION_KIND),
            self.purpose,
        )
        .await;
        let policy =
            EndpointPolicy::from_login_servers(target.servers.iter().map(|s| s.url.as_str()));
        let login_server = target
            .servers
            .first()
            .map_or("", |server| server.url.as_str());
        let live = minted.map_err(mint_reason).and_then(|session| {
            let url = session::upload_for(
                &policy,
                login_server,
                &session.marker().telemetry.upload_endpoint,
                LAUNCHER_SESSION_KIND,
            )?;
            Ok(Live {
                session,
                upload_url: url,
            })
        });
        match live {
            Ok(live) => {
                *locked(&self.mint_failures) = 0;
                *locked(&self.session) = Some(live.clone());
                Ok(live)
            }
            Err(reason) => {
                let failures = {
                    let mut count = locked(&self.mint_failures);
                    *count = count.saturating_add(1);
                    *count
                };
                let doubled = MINT_HOLD.saturating_mul(2u32.saturating_pow(failures - 1));
                self.hold(doubled.min(MAX_HOLD));
                Err(reason)
            }
        }
    }

    async fn post(&self, live: &Live, batch: &[TelemetryEvent]) -> Result<(), ChunkError> {
        let block = &live.session.marker().telemetry;
        chunk::post_chunk(&self.http, &live.upload_url, &block.token, batch).await
    }

    fn held(&self) -> bool {
        locked(&self.hold_until).is_some_and(|until| until > Instant::now())
    }

    fn hold(&self, wait: Duration) {
        *locked(&self.hold_until) = Some(Instant::now() + wait);
    }

    /// Forgets the stored session so the next post mints a new one. The hold from a failed mint stays.
    pub fn end_session(&self) {
        *locked(&self.session) = None;
    }

    /// The time left on the hold, if any.
    #[cfg(test)]
    pub fn hold_remaining(&self) -> Option<Duration> {
        locked(&self.hold_until).and_then(|until| until.checked_duration_since(Instant::now()))
    }

    /// Lifts the hold, for tests that send again at once.
    #[cfg(test)]
    pub fn clear_hold(&self) {
        *locked(&self.hold_until) = None;
    }
}

fn locked<T>(slot: &Mutex<T>) -> MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

/// True when the session has less than `REMINT_MARGIN_MS` left.
fn expiring(session: &Session) -> bool {
    let left = session.marker().telemetry.expires_at_ms - chrono::Utc::now().timestamp_millis();
    left < REMINT_MARGIN_MS
}

/// The C8 reason the session module recorded for a failed mint.
fn mint_reason(failure: MintFailure) -> &'static str {
    failure.reason
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_failures_keep_the_c8_reason() {
        let failure = MintFailure {
            outcome: Outcome::SessionUnavailable,
            reason: "http_status",
        };
        assert_eq!(mint_reason(failure), "http_status");
    }
}

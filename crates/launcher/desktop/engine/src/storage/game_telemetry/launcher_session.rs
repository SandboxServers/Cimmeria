//! The launcher's own adoption events, sent at once under a `launcher` session
//! (C11). Nothing is queued on disk and the upload queue is never touched: a
//! send that cannot post drops its events (README D-AU21).
use super::session::{self, Session};
use super::*;
use crate::client_setup::login_servers::{self, LoginServer};
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
/// Rows per chunk; the server keeps at most this many (C14).
pub const LAUNCHER_MAX_ROWS: usize = 200;
/// Re-mint when the token has less than this left.
pub const REMINT_MARGIN_MS: i64 = 60_000;

const LAUNCHER_TAGS: [&str; 3] = ["desktop-launcher", std::env::consts::OS, "launcher"];

/// What a launcher send needs: the opted-in identity and the login servers to mint on.
#[derive(Debug, Clone)]
pub struct LauncherTarget {
    pub identity: Identity,
    pub servers: Vec<LoginServer>,
}

impl DesktopState {
    /// `Some` only while the player is opted in to game diagnostics and has an
    /// identity. `servers` is the saved login-server preference, or the defaults
    /// when it is empty, validated. Never mints, never sends.
    pub fn launcher_target(&self) -> Option<LauncherTarget> {
        let choice = self.game_telemetry().ok()?;
        let identity = choice.opted_in.then_some(choice.identity).flatten()?;
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
    Empty,
    Sent,
    Held,
    MintFailed,
    Dropped,
}

/// Sends launcher events over one HTTP client, keeping one session and one hold.
pub struct LauncherSender {
    http: reqwest::Client,
    session: Mutex<Option<Session>>,
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
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(session::CONNECT)
            .timeout(session::TOTAL)
            .build()
            .expect("the TLS backend is compiled in");
        Self {
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
            return SendOutcome::Held;
        }
        let Some(mut current) = self.usable_session(target).await else {
            return SendOutcome::MintFailed;
        };
        let max_bytes = current.marker().telemetry.chunk_max_bytes;
        let Ok(batches) = chunk::split_batches(events, max_bytes, LAUNCHER_MAX_ROWS) else {
            return SendOutcome::Dropped;
        };
        for range in batches {
            let batch = &events[range];
            let mut posted = self.post(&current, batch).await;
            if matches!(posted, Err(ChunkError::TokenRejected)) {
                // A 401 is the only error that mints again, and only once.
                *locked(&self.session) = None;
                let Some(fresh) = self.mint(target).await else {
                    return SendOutcome::Dropped;
                };
                current = fresh;
                posted = self.post(&current, batch).await;
            }
            match posted {
                Ok(()) => {}
                Err(err @ (ChunkError::KillSwitch { .. } | ChunkError::RateLimited { .. })) => {
                    let asked = err.retry_after_secs().unwrap_or(60);
                    self.hold(Duration::from_secs(asked).clamp(MIN_RETRY_HOLD, MAX_HOLD));
                    return SendOutcome::Dropped;
                }
                Err(_) => return SendOutcome::Dropped,
            }
        }
        SendOutcome::Sent
    }

    /// The stored session while it has time left, else a fresh mint.
    async fn usable_session(&self, target: &LauncherTarget) -> Option<Session> {
        let stored = locked(&self.session).clone();
        match stored {
            Some(session) if !expiring(&session) => Some(session),
            _ => self.mint(target).await,
        }
    }

    /// Mint a launcher session. A failure holds the sender, doubling per failure
    /// in a row up to `MAX_HOLD`.
    async fn mint(&self, target: &LauncherTarget) -> Option<Session> {
        let minted = session::start_kind(
            &target.identity,
            &target.servers,
            &LAUNCHER_TAGS,
            Some(LAUNCHER_SESSION_KIND),
        )
        .await;
        match minted {
            Ok(minted) => {
                *locked(&self.mint_failures) = 0;
                *locked(&self.session) = Some(minted.clone());
                Some(minted)
            }
            Err(_) => {
                let failures = {
                    let mut count = locked(&self.mint_failures);
                    *count = count.saturating_add(1);
                    *count
                };
                let doubled = MINT_HOLD.saturating_mul(2u32.saturating_pow(failures - 1));
                self.hold(doubled.min(MAX_HOLD));
                None
            }
        }
    }

    async fn post(&self, session: &Session, batch: &[TelemetryEvent]) -> Result<(), ChunkError> {
        let block = &session.marker().telemetry;
        chunk::post_chunk(&self.http, &block.upload_endpoint, &block.token, batch).await
    }

    fn held(&self) -> bool {
        locked(&self.hold_until).is_some_and(|until| until > Instant::now())
    }

    fn hold(&self, wait: Duration) {
        *locked(&self.hold_until) = Some(Instant::now() + wait);
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

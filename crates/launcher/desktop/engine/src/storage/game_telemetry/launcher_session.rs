//! The launcher's own adoption events, sent at once under a `launcher` session
//! (C11). Nothing is queued on disk and the upload queue is never touched: a
//! send that cannot post drops its events (README D-AU21).
use super::endpoint_sanity::effective_upload_endpoint;
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

/// Sends launcher events over one HTTP client, keeping one session and one hold.
pub struct LauncherSender {
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
        )
        .await;
        let login_server = target
            .servers
            .first()
            .map_or("", |server| server.url.as_str());
        let live = minted
            .map_err(mint_reason)
            .and_then(|session| upload_for(login_server, session));
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

/// The C8 reason for a failed mint. A plain transport or reply failure is
/// `transport`; the session module does not split it further.
fn mint_reason(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::EndpointRefused => "endpoint_refused",
        _ => "transport",
    }
}

/// The upload URL for a minted session (C10). When the advertised address is
/// replaced, one info is logged for this session. `Err` is a C8 reason.
fn upload_for(login_server: &str, session: Session) -> Result<Live, &'static str> {
    let advertised = &session.marker().telemetry.upload_endpoint;
    let effective =
        effective_upload_endpoint(login_server, advertised).ok_or("no_https_endpoint")?;
    if effective.overridden {
        endpoint_overridden();
    }
    Ok(Live {
        session,
        upload_url: effective.url,
    })
}

/// The advertised loopback upload address was replaced by the login server's own
/// `/api/telemetry` (launcher-telemetry C10). Logged once per minted session.
fn endpoint_overridden() {
    tracing::info!(
        target: "launcher.telemetry",
        event = "upload_endpoint_overridden",
        kind = "launcher",
        "launcher telemetry: the server advertised a loopback upload address; expected its public address; using the login server's /api/telemetry instead"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;

    fn overridden_events(captured: &[crate::test_capture::Captured]) -> usize {
        captured
            .iter()
            .filter(|c| c.target == "launcher.telemetry")
            .filter(|c| c.field("event") == Some("upload_endpoint_overridden"))
            .count()
    }

    #[test]
    fn a_loopback_advertised_endpoint_from_an_https_login_is_replaced_and_logged_once() {
        let (live, captured) = capture(|| upload_for("https://play.example", Session::fixture()));
        let live = live.expect("an https login gives a URL");
        assert_eq!(live.upload_url, "https://play.example/api/telemetry");
        assert_eq!(overridden_events(&captured), 1);
        assert_eq!(captured[0].field("kind"), Some("launcher"));
    }

    #[test]
    fn a_plain_http_login_with_a_loopback_advertised_endpoint_gives_no_url() {
        let (reason, captured) =
            capture(|| upload_for("http://play.example:8081", Session::fixture()).err());
        assert_eq!(reason, Some("no_https_endpoint"));
        assert_eq!(overridden_events(&captured), 0);
    }

    #[test]
    fn a_loopback_login_keeps_the_advertised_endpoint_without_a_log() {
        let (live, captured) = capture(|| upload_for("http://127.0.0.1:8081", Session::fixture()));
        let live = live.expect("a loopback login keeps its endpoint");
        assert_eq!(live.upload_url, "http://localhost:8081/api/telemetry");
        assert_eq!(overridden_events(&captured), 0);
    }

    #[test]
    fn mint_failures_map_to_c8_reasons() {
        assert_eq!(mint_reason(Outcome::EndpointRefused), "endpoint_refused");
        assert_eq!(mint_reason(Outcome::SessionUnavailable), "transport");
    }
}

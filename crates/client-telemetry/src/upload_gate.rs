//! When the uploader may POST, and with which token.
//!
//! The retry decision for refused and failed uploads, kept free of HTTP, the
//! file system and the clock: every method takes the monotonic `now`, the
//! wall-clock `wall_ms` and, where it needs one, a `reload` callback that
//! re-reads `current-session.json`. The uploader loop in [`crate::uploader`]
//! feeds it the outcome of each POST.
//!
//! # Why
//!
//! On 2026-10-12 a lab client ran for 8 h after its last lease. Its token
//! expired, and the uploader retried every flush with it: about three
//! refused POSTs every 12 s for 4.5 h, each one a `token_expired` 401 in the
//! colo's logs and Discord. The gate stops that:
//!
//! - **Auth refusal (401, 403).** Re-read the session file. A different
//!   token is swapped in and retried at once. The same token, or no readable file,
//!   holds uploads off with exponential backoff: the flush interval, then
//!   doubling, capped at [`BACKOFF_CAP`]. Each backoff deadline re-reads the
//!   file again, so a later rotation recovers.
//! - **A token known to be unusable** (empty, or past its `expires_at_ms`)
//!   is never sent. The gate holds and re-reads on the same backoff instead.
//! - **5xx, other refusals and network errors.** The same bounded backoff,
//!   or the server's `Retry-After` when that is longer (also capped).
//!
//! Who writes a new token: nobody, yet, while the client runs. The lab
//! supervisor writes the file once per launch. The launchers refresh their
//! token at 75% TTL (`launcher-core` `Telemetry::refresh_if_due`, the
//! desktop engine's transport), but keep the new one in memory and do not
//! rewrite the file (as of 2026-10-12). So today a DLL stops uploading
//! quietly once its first token expires; the re-read is what lets it
//! recover as soon as a launcher does rewrite the file.
//!
//! Pausing for auth and resuming afterwards are reported once each, as
//! [`Transition`]s the uploader turns into one event and one log line; a
//! backoff attempt itself reports nothing.

use std::time::{Duration, Instant};

/// Longest hold between two upload attempts, whatever the cause.
pub const BACKOFF_CAP: Duration = Duration::from_secs(600);

/// The token half of `current-session.json`'s `telemetry` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub token: String,
    /// Wall-clock expiry in ms since the epoch. `0` (or below) means the
    /// file did not say, which never counts as expired.
    pub expires_at_ms: i64,
}

impl Credentials {
    /// Worth sending at wall-clock `wall_ms`: non-empty and not known to
    /// have expired.
    pub fn usable_at(&self, wall_ms: i64) -> bool {
        !self.token.is_empty() && (self.expires_at_ms <= 0 || self.expires_at_ms > wall_ms)
    }
}

/// Why uploads paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseReason {
    /// The server refused the token (401 or 403).
    Refused,
    /// The token's `expires_at_ms` has passed; it was not sent.
    Expired,
    /// The session file carries no token (a lab launch whose mint failed).
    NoToken,
}

impl PauseReason {
    pub fn as_str(self) -> &'static str {
        match self {
            PauseReason::Refused => "refused",
            PauseReason::Expired => "expired",
            PauseReason::NoToken => "no_token",
        }
    }
}

/// A change in the gate's state worth one report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    /// Uploads stopped for want of a usable token. `status` is the server's
    /// answer when it refused one; `retry_in` the first hold.
    Paused {
        reason: PauseReason,
        status: Option<u16>,
        retry_in: Duration,
    },
    /// A POST went through again after a pause. `attempts` counts the
    /// holds in between.
    Resumed { paused_for: Duration, attempts: u32 },
}

/// What the uploader does after an auth refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterRefusal {
    /// The session file carried a new token: POST again now with it.
    RetryNow,
    /// No new token: nothing is sent for this long.
    Hold(Duration),
}

/// The upload retry state machine. See the module docs.
#[derive(Debug)]
pub struct UploadGate {
    creds: Credentials,
    base: Duration,
    cap: Duration,
    /// Consecutive failures since the last success or token swap.
    failures: u32,
    hold_until: Option<Instant>,
    /// Set while uploads are paused for auth.
    paused_since: Option<Instant>,
    pause_attempts: u32,
    transitions: Vec<Transition>,
}

impl UploadGate {
    /// A gate starting from `creds`. `base` is the first hold (the
    /// uploader passes its flush interval); holds double up to `cap`.
    pub fn new(creds: Credentials, base: Duration, cap: Duration) -> Self {
        Self {
            creds,
            base: base.max(Duration::from_millis(1)),
            cap,
            failures: 0,
            hold_until: None,
            paused_since: None,
            pause_attempts: 0,
            transitions: Vec::new(),
        }
    }

    /// The token to put in `Authorization: Bearer`.
    pub fn token(&self) -> &str {
        &self.creds.token
    }

    /// Whether uploads are paused for auth.
    pub fn is_paused(&self) -> bool {
        self.paused_since.is_some()
    }

    /// The hold after `n` consecutive failures: `base * 2^(n-1)`, capped.
    pub fn delay_for(&self, n: u32) -> Duration {
        let shift = n.saturating_sub(1).min(31);
        self.base.saturating_mul(1u32 << shift).min(self.cap)
    }

    /// Transitions since the last call, oldest first.
    pub fn take_transitions(&mut self) -> Vec<Transition> {
        std::mem::take(&mut self.transitions)
    }

    /// May a POST go now? `false` while a hold runs. Past a hold, while
    /// paused for auth or holding a token known to be unusable, the file is
    /// re-read (`reload`): a different token is adopted, and an unusable one
    /// starts the next, longer hold instead of a POST.
    pub fn ready(
        &mut self,
        now: Instant,
        wall_ms: i64,
        reload: impl FnOnce() -> Option<Credentials>,
    ) -> bool {
        if self.hold_until.is_some_and(|t| now < t) {
            return false;
        }
        if self.paused_since.is_some() || !self.creds.usable_at(wall_ms) {
            self.adopt_if_new(reload());
            if !self.creds.usable_at(wall_ms) {
                let reason = if self.creds.token.is_empty() {
                    PauseReason::NoToken
                } else {
                    PauseReason::Expired
                };
                let d = self.hold(now);
                self.pause(now, reason, None, d);
                return false;
            }
        }
        self.hold_until = None;
        true
    }

    /// The last POST before exit ignores backoff holds (the process is
    /// leaving; one request is the most it costs) but still never sends a
    /// token known to be unusable.
    pub fn ready_for_final(
        &mut self,
        wall_ms: i64,
        reload: impl FnOnce() -> Option<Credentials>,
    ) -> bool {
        if self.paused_since.is_some() || !self.creds.usable_at(wall_ms) {
            self.adopt_if_new(reload());
        }
        self.creds.usable_at(wall_ms)
    }

    /// A POST was accepted.
    pub fn on_success(&mut self, now: Instant) {
        self.failures = 0;
        self.hold_until = None;
        if let Some(since) = self.paused_since.take() {
            self.transitions.push(Transition::Resumed {
                paused_for: now.saturating_duration_since(since),
                attempts: self.pause_attempts,
            });
            self.pause_attempts = 0;
        }
    }

    /// The server refused the token with `status`. Re-reads the file: a new
    /// usable token means retry now, anything else a hold.
    pub fn on_auth_refused(
        &mut self,
        now: Instant,
        wall_ms: i64,
        status: u16,
        reload: impl FnOnce() -> Option<Credentials>,
    ) -> AfterRefusal {
        if self.adopt_if_new(reload()) && self.creds.usable_at(wall_ms) {
            self.hold_until = None;
            return AfterRefusal::RetryNow;
        }
        let d = self.hold(now);
        self.pause(now, PauseReason::Refused, Some(status), d);
        AfterRefusal::Hold(d)
    }

    /// A POST failed for another reason (5xx, another 4xx, network). Holds
    /// for the backoff, or for `retry_after` when the server asked for
    /// longer; both capped. Returns the hold.
    pub fn on_transient(&mut self, now: Instant, retry_after: Option<Duration>) -> Duration {
        let d = self.hold(now);
        let d = match retry_after {
            Some(ra) if ra > d => ra.min(self.cap),
            _ => d,
        };
        self.hold_until = Some(now + d);
        d
    }

    /// Adopt `fresh` when it carries a different token. Returns whether it
    /// did. A swap resets the backoff: the new token gets a prompt try.
    fn adopt_if_new(&mut self, fresh: Option<Credentials>) -> bool {
        match fresh {
            Some(c) if c.token != self.creds.token => {
                self.creds = c;
                self.failures = 0;
                true
            }
            Some(c) => {
                // Same token, perhaps a corrected expiry.
                self.creds.expires_at_ms = c.expires_at_ms;
                false
            }
            None => false,
        }
    }

    fn hold(&mut self, now: Instant) -> Duration {
        self.failures = self.failures.saturating_add(1);
        let d = self.delay_for(self.failures);
        self.hold_until = Some(now + d);
        d
    }

    fn pause(
        &mut self,
        now: Instant,
        reason: PauseReason,
        status: Option<u16>,
        retry_in: Duration,
    ) {
        if self.paused_since.is_none() {
            self.paused_since = Some(now);
            self.pause_attempts = 0;
            self.transitions.push(Transition::Paused {
                reason,
                status,
                retry_in,
            });
        }
        self.pause_attempts = self.pause_attempts.saturating_add(1);
    }
}

#[cfg(test)]
#[path = "upload_gate_tests.rs"]
mod tests;

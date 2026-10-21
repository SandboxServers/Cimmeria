//! Game-telemetry transport events (launcher-telemetry LT-09a; C5, C6).
use super::TransportError;
use cimmeria_launcher_core::telemetry::auth::AuthError;
use cimmeria_launcher_core::telemetry::bundle::BundleError;
use cimmeria_launcher_core::telemetry::chunk::ChunkError;
use cimmeria_launcher_core::telemetry::TelemetryError;
use std::sync::atomic::{AtomicU64, Ordering};

/// The closed code for a transport error (C6 `reason`). No wildcard arm.
pub(crate) fn reason(code: TransportError) -> &'static str {
    match code {
        TransportError::NotOptedIn => "not_opted_in",
        TransportError::Choice => "choice_unreadable",
        TransportError::SessionUnavailable => "session_unavailable",
        TransportError::EndpointRefused => "endpoint_refused",
        TransportError::Queue => "queue",
        TransportError::TokenRejected => "token_rejected",
        TransportError::BackingOff => "backing_off",
        TransportError::Upload => "upload",
    }
}

/// The HTTP status behind a raw core error, if the server answered.
pub(crate) fn http_status(error: &TelemetryError) -> Option<u16> {
    match error {
        TelemetryError::Auth(AuthError::Status { status, .. })
        | TelemetryError::Chunk(ChunkError::Status { status, .. })
        | TelemetryError::Chunk(ChunkError::Rejected { status, .. })
        | TelemetryError::Bundle(BundleError::Status { status, .. }) => Some(*status),
        TelemetryError::Chunk(ChunkError::TokenRejected)
        | TelemetryError::Bundle(BundleError::TokenRejected) => Some(401),
        TelemetryError::Chunk(ChunkError::RateLimited { .. }) => Some(429),
        TelemetryError::Auth(AuthError::KillSwitch { .. })
        | TelemetryError::Chunk(ChunkError::KillSwitch { .. })
        | TelemetryError::Bundle(BundleError::KillSwitch { .. }) => Some(503),
        _ => None,
    }
}

/// Consecutive failures of one kind. The first is forwarded at warn; repeats
/// stay local at debug, so an outage writes one forwarded row, not one per flush.
#[derive(Debug, Default)]
pub(super) struct Streak(AtomicU64);

impl Streak {
    /// Count one more failure; returns the new count (1 for the first).
    pub(super) fn fail(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Clear the count; returns what it was.
    pub(super) fn reset(&self) -> u64 {
        self.0.swap(0, Ordering::SeqCst)
    }
}

#[derive(Debug, Default)]
pub(super) struct Streaks {
    pub(super) upload: Streak,
    pub(super) refresh: Streak,
}

/// A chunk flush the server did not take. Warn for the first of a streak,
/// debug for the repeats.
pub(super) fn chunk_upload_failed(streak: &Streak, error: &TelemetryError) {
    let attempt = streak.fail();
    let code = reason(TransportError::of(error));
    let status = http_status(error);
    if attempt == 1 {
        tracing::warn!(
            target: "launcher.telemetry",
            event = "chunk_upload_failed",
            reason = code,
            http_status = status,
            attempt,
            "game telemetry: telemetry flush failed; expected the server to take the queued chunks; they stay queued and the next flush retries"
        );
    } else {
        tracing::debug!(
            target: "launcher.telemetry",
            event = "chunk_upload_failed",
            reason = code,
            http_status = status,
            attempt,
            "game telemetry: telemetry flush failed; expected the server to take the queued chunks; they stay queued and the next flush retries"
        );
    }
}

/// The first accepted flush after a streak of failures. Nothing when the
/// streak was already clear.
pub(super) fn chunk_upload_resumed(streak: &Streak) {
    let count = streak.reset();
    if count > 0 {
        tracing::info!(
            target: "launcher.telemetry",
            event = "chunk_upload_resumed",
            count,
            "game telemetry: chunk uploads resumed after failed flushes; the events they held are being sent"
        );
    }
}

/// The upload endpoint refused the token; a renewal follows.
pub(super) fn token_renew_started() {
    tracing::info!(
        target: "launcher.telemetry",
        event = "token_renew_started",
        reason = "token_rejected",
        "game telemetry: the upload endpoint refused the token; renewing it"
    );
}

/// A renewal or re-mint that gave no working token; `held` is always true.
pub(super) fn token_refresh_failed(
    action: &'static str,
    code: TransportError,
    status: Option<u16>,
) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "token_refresh_failed",
        action,
        reason = reason(code),
        http_status = status,
        held = true,
        "game telemetry: telemetry token not renewed; expected a working token from the login server; flushes hold off and retry later"
    );
}

/// The proactive refresh failed; the current token stays in use. Warn for
/// the first of a streak, debug for the repeats.
pub(super) fn proactive_refresh_failed(streak: &Streak, code: TransportError, status: Option<u16>) {
    let attempt = streak.fail();
    let why = reason(code);
    if attempt == 1 {
        tracing::warn!(
            target: "launcher.telemetry",
            event = "token_refresh_failed",
            action = "refresh_proactive",
            reason = why,
            http_status = status,
            held = false,
            attempt,
            "game telemetry: proactive token refresh failed; expected a fresh token before this one expires; the current token stays in use"
        );
    } else {
        tracing::debug!(
            target: "launcher.telemetry",
            event = "token_refresh_failed",
            action = "refresh_proactive",
            reason = why,
            http_status = status,
            held = false,
            attempt,
            "game telemetry: proactive token refresh failed; expected a fresh token before this one expires; the current token stays in use"
        );
    }
}

/// A new session was minted after the server refused the old token.
pub(super) fn session_reminted() {
    tracing::info!(
        target: "launcher.telemetry",
        event = "session_reminted",
        reason = "token_rejected",
        "game telemetry: minted a new telemetry session after the server refused the old token; uploads continue on it"
    );
}

/// The end-of-session bundle was not taken.
pub(super) fn bundle_upload_failed(error: &TelemetryError) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "bundle_upload_failed",
        reason = reason(TransportError::of(error)),
        http_status = http_status(error),
        "game telemetry: session log bundle not sent; expected the server to take it; this session's logs are not uploaded"
    );
}

/// Lines the disk queue dropped since the last bundle.
pub(super) fn queue_dropped(count: u64) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "queue_dropped",
        count,
        "game telemetry: the disk queue dropped lines since the last bundle (queue cap or chunks the server refused); those events were never sent"
    );
}

/// The dropped-lines counter could not be cleared after a bundle.
pub(super) fn dropped_counter_reset_failed() {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "dropped_counter_reset_failed",
        reason = "queue",
        "game telemetry: dropped-lines counter not reset after the bundle; expected it cleared; the next bundle reports these lines again"
    );
}

/// The queue was deleted unsent after the player's choice changed.
pub(super) fn queue_discarded() {
    tracing::info!(
        target: "launcher.telemetry",
        event = "queue_discarded",
        "game telemetry: queue deleted unsent after the game-telemetry choice changed"
    );
}

/// The queue could not be deleted; `action` names the write that found it.
pub(super) fn queue_discard_failed(action: &'static str) {
    tracing::warn!(
        target: "launcher.telemetry",
        event = "queue_discard_failed",
        action,
        reason = "queue",
        "game telemetry: queue not deleted; expected it gone after the choice changed; it is retried at the next change or startup"
    );
}

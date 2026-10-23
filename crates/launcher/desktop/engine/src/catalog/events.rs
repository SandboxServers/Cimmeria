//! Release catalog events (launcher-telemetry LT-13; C5, C6). A fetch the player
//! sees fail is one warn row with a closed `reason`. No URL, host or error text
//! is written (R5).
use std::time::Duration;

/// A release fetch failed. `reason` is `network`, `http_status`, `body`,
/// `signature`, `invalid_manifest`, `signing_key_unavailable` or
/// `development_key`; `http_status` is set only for `http_status`.
pub(crate) fn release_fetch_failed(reason: &'static str, http_status: Option<u16>) {
    match http_status {
        Some(http_status) => tracing::warn!(
            target: "launcher.game_update",
            event = "release_fetch_failed",
            reason,
            http_status,
            "game update: the release manifest fetch failed; expected a signed manifest; no update is offered"
        ),
        None => tracing::warn!(
            target: "launcher.game_update",
            event = "release_fetch_failed",
            reason,
            "game update: the release manifest fetch failed; expected a signed manifest; no update is offered"
        ),
    }
}

/// The release manifest and its signature were fetched and verified.
pub(crate) fn release_fetched(elapsed: Duration) {
    let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    tracing::info!(
        target: "launcher.game_update",
        event = "release_fetched",
        duration_ms,
        "game update: the release manifest was fetched and verified; expected a verified release; the offer can be shown"
    );
}

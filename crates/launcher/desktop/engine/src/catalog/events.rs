//! Release catalog events (launcher-telemetry LT-13; C5, C6). A fetch the player
//! sees fail is one warn row with a closed `reason`. No URL, host or error text
//! is written (R5).
use super::Purpose;
use std::time::Duration;

/// A release fetch failed. `purpose` names why it was fetched (`install`,
/// `update`, `adopt`, `seed`, `preflight`). `reason` is `network`,
/// `http_status`, `body`, `signature`, `invalid_manifest`,
/// `signing_key_unavailable` or `development_key`; `http_status` is set only
/// for `http_status`.
pub(crate) fn release_fetch_failed(
    purpose: Purpose,
    reason: &'static str,
    http_status: Option<u16>,
) {
    let (label, next) = wording(purpose);
    let purpose = purpose.code();
    match http_status {
        Some(http_status) => tracing::warn!(
            target: "launcher.game_update",
            event = "release_fetch_failed",
            purpose,
            reason,
            http_status,
            "{label}: the release manifest fetch failed; expected a signed manifest; {next}"
        ),
        None => tracing::warn!(
            target: "launcher.game_update",
            event = "release_fetch_failed",
            purpose,
            reason,
            "{label}: the release manifest fetch failed; expected a signed manifest; {next}"
        ),
    }
}

/// The message's subject and its "what happens next" clause, literals only.
fn wording(purpose: Purpose) -> (&'static str, &'static str) {
    match purpose {
        Purpose::Update => ("game update", "no update is offered"),
        Purpose::Install => ("install", "the install does not start"),
        Purpose::Adopt => ("adoption", "the folder is not adopted"),
        Purpose::Seed => ("seed archive", "the seed archive is not used"),
        Purpose::Preflight => ("preflight", "the release check reports the failure"),
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

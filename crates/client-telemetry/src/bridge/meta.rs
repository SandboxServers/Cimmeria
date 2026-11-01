//! Frame stamp on bridge responses built by the main-thread drain (lab
//! autopilot contract C4). It lets the daemon tell a read taken after an
//! action from one taken in the same frame.
//!
//! Responses built off the main thread (queue full, parse errors) carry no
//! stamp: they never ran a method, so there is no frame to attest to.

use std::sync::OnceLock;
use std::time::Instant;

use serde::Serialize;

/// Stamp attached to a drained response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Meta {
    /// Bridge tick counter (what `heartbeat` reports) after the method ran.
    pub frame: u64,
    /// Milliseconds since the bridge started.
    pub mono_ms: u64,
    /// Game clock in seconds; absent when the clock cannot be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_t: Option<f64>,
}

static START: OnceLock<Instant> = OnceLock::new();

/// Record the bridge start time. The first call wins.
pub(crate) fn mark_start() {
    let _ = START.set(Instant::now());
}

/// Milliseconds since [`mark_start`]; 0 when it was never called.
pub(crate) fn mono_ms() -> u64 {
    START.get().map_or(0, |s| {
        u64::try_from(s.elapsed().as_millis()).unwrap_or(u64::MAX)
    })
}

/// Build the stamp for a response being finished right now.
pub(crate) fn now(game_t: Option<f64>) -> Meta {
    Meta {
        frame: super::heartbeat_count(),
        mono_ms: mono_ms(),
        game_t,
    }
}

/// The game clock through the existing checked reader; `None` off the
/// client (host tests) or when the connection is not up yet.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) fn live_game_t() -> Option<f64> {
    crate::hooks::ability_trace::clock::game_time(&crate::hooks::entity_trace::map::LiveMem)
}

#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub(crate) fn live_game_t() -> Option<f64> {
    None
}

#[cfg(test)]
#[path = "meta_tests.rs"]
pub(crate) mod tests;

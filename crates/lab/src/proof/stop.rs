//! Stop reports (C8): a world tool that gives up says why, what it tried and
//! where the player was, so the caller can pick a recipe instead of retrying.

use serde::Serialize;

/// Why a world tool stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopKind {
    Stuck,
    Timeout,
    IgnoredInput,
    Stalled,
    BlockedUi,
    Ledge,
}

impl StopKind {
    /// The snake_case name used in JSON and in the failure message.
    pub fn as_str(self) -> &'static str {
        match self {
            StopKind::Stuck => "stuck",
            StopKind::Timeout => "timeout",
            StopKind::IgnoredInput => "ignored_input",
            StopKind::Stalled => "stalled",
            StopKind::BlockedUi => "blocked_ui",
            StopKind::Ledge => "ledge",
        }
    }
}

/// A stop: the kind, the step it stopped in and what was tried.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StopReport {
    pub kind: StopKind,
    pub step: String,
    pub seconds: f64,
    /// What the tool already tried, in order.
    pub tried: Vec<String>,
    /// Where the player was and is.
    pub player: Option<StopPlayer>,
    /// C11 recipe names, best first.
    pub suggest: Vec<String>,
}

/// The player's position when the step began and now, in server metres.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StopPlayer {
    pub at: [f64; 3],
    pub now: [f64; 3],
    pub moved_m: f64,
}

impl StopPlayer {
    /// Build the player block; `moved_m` is the straight-line distance from `at` to `now`.
    pub fn new(at: [f64; 3], now: [f64; 3]) -> Self {
        let d = [now[0] - at[0], now[1] - at[1], now[2] - at[2]];
        let moved_m = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        Self { at, now, moved_m }
    }
}

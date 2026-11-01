//! Ignored input versus a blocking window (AX-34). Pure over plain values.

use crate::proof::stop::StopKind;
use crate::proof::{Check, Source, Why};
use serde_json::Value;

/// Windows that are always up in the world and do not take the keyboard from
/// the pawn. Anything else visible (a dialog, a vendor, a map) may swallow the
/// movement keys, so the stop is `blocked_ui`.
pub(super) const HUD_WINDOWS: &[&str] = &[
    "SelfStatusWin",
    "BackgroundWin",
    "MissionTrackerWin",
    "Inst1ChatWin",
    "ChatWin",
    "MinimapWin",
    "SCTWin",
    "TargetStatusWin",
    "HotbarWin",
];

/// `BlockedUi` when a visible window is not part of the HUD, else the input
/// was simply ignored.
pub(super) fn classify(visible_windows: &[String], hud: &[&str]) -> StopKind {
    if visible_windows.iter().any(|w| !hud.contains(&w.as_str())) {
        StopKind::BlockedUi
    } else {
        StopKind::IgnoredInput
    }
}

/// Synthetic mouse records the DLL handed to a buffered read
/// (`hooks.events_delivered`). Not `data_reads_mouse`: that counts every poll,
/// delivered or not. `None` for an old DLL (null `hooks`).
pub(in crate::supervisor) fn delivery_count(hooks: &Value) -> Option<u64> {
    hooks.get("events_delivered")?.as_u64()
}

/// Did the game take any synthetic mouse records across a look? An old DLL (`None`) never
/// denies: the check is unmeasured.
pub(super) fn look_delivered(before: Option<u64>, after: Option<u64>) -> Check {
    match (before, after) {
        (Some(b), Some(a)) => Check::new("look_delivered", Source::Bridge, Some(a > b), true)
            .with_before(serde_json::json!(b))
            .with_after(serde_json::json!(a))
            .with_why(Why::NotDelivered),
        _ => Check::new("look_delivered", Source::Bridge, None, true).with_why(Why::NoPostRead),
    }
}

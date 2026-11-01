//! Ignored input versus a blocking window (AX-34). Pure over plain values.

use crate::proof::stop::StopKind;
use crate::proof::{Check, Source, Why};

/// Windows that are always up in the world and do not take the keyboard from
/// the pawn. Anything else visible (a dialog, a vendor, a map) may swallow the
/// movement keys, so the stop is `blocked_ui`.
pub(super) const HUD_WINDOWS: &[&str] = &[
    "SelfStatusWin",
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

/// Did the DirectInput counters move across a look? An old DLL (`None`) never
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

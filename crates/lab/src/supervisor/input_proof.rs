//! Input primitives say what they know (AX-43). Pure over plain values: a
//! posted key cannot be read back, while a cursor move, a focus change and a
//! release can. The mouse counter is `hooks.events_delivered` (AX-34): the
//! synthetic records the DLL handed to a buffered read.

use serde_json::{json, Value};

use crate::proof::{Check, EvVerdict, Evidence, Source, Why};
use crate::supervisor::world::ignored_input::delivery_count;

/// What one `client_input_mouse` call asked for and what it read back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MouseFacts {
    /// DirectInput relative horizontal motion.
    pub dx: i32,
    /// DirectInput relative vertical motion.
    pub dy: i32,
    /// DirectInput wheel delta.
    pub wheel: i32,
    /// A button press, release or click was asked for.
    pub button: bool,
    /// `Some(true)` when the call put the UI cursor on its point first.
    pub cursor_placed: Option<bool>,
    /// `hooks.events_delivered` before the call.
    pub di_before: Option<u64>,
    /// `hooks.events_delivered` after the settle poll.
    pub di_after: Option<u64>,
}

/// A posted key (or button): the game's use of it cannot be read back.
pub fn key_evidence() -> Evidence {
    Evidence::from_verdict(EvVerdict::Inconclusive, Why::PostedOnly, Vec::new())
}

/// The verdict for one `client_input_mouse` call. A button is posted only;
/// motion is delivered when the counter rose; a cursor-only move is read back.
pub fn mouse_evidence(m: MouseFacts) -> Evidence {
    if m.button {
        return key_evidence();
    }
    if m.dx != 0 || m.dy != 0 || m.wheel != 0 {
        return match (m.di_before, m.di_after) {
            (Some(b), Some(a)) => {
                let check = Check::new("di_delivered", Source::Bridge, Some(a > b), true)
                    .with_before(json!(b))
                    .with_after(json!(a))
                    .with_why(Why::NotDelivered);
                if check.ok == Some(true) {
                    Evidence::from_verdict(EvVerdict::Inconclusive, Why::Delivered, vec![check])
                } else {
                    Evidence::judge(vec![check])
                }
            }
            _ => key_evidence(),
        };
    }
    match m.cursor_placed {
        Some(placed) => Evidence::judge(vec![Check::new(
            "cursor_at",
            Source::Client,
            Some(placed),
            true,
        )]),
        None => Evidence::no_evidence(),
    }
}

/// `hooks.events_delivered` from an `input_status` reply. `None` for an old
/// DLL (null `hooks`).
pub fn di_count(status: &Value) -> Option<u64> {
    delivery_count(&status["hooks"])
}

/// A cursor move: confirmed when `place_cursor` read the point back.
pub fn cursor_evidence(placed: bool, at: (i32, i32)) -> Evidence {
    Evidence::judge(vec![Check::new(
        "cursor_at",
        Source::Client,
        Some(placed),
        true,
    )
    .with_after(json!([at.0, at.1]))])
}

/// A focus change: confirmed when the game's virtual focus window matches
/// the request (`virtual_focus_hwnd` set for on, null for off).
pub fn focus_evidence(want_on: bool, hwnd_after: Option<u64>) -> Evidence {
    let check = Check::new(
        "virtual_focus",
        Source::Bridge,
        Some(hwnd_after.is_some() == want_on),
        true,
    )
    .with_after(json!(hwnd_after));
    Evidence::judge(vec![check])
}

/// A release: confirmed when `input_status` shows no held key or button
/// (`held.keys`, `held.buttons`) and no C5 hold (`holds`, or absent).
pub fn release_evidence(held_after: &Value, holds_after: Option<&Value>) -> Evidence {
    let held = ["keys", "buttons"].map(|k| held_after.get(k).and_then(Value::as_array));
    let held_ok = match held {
        [Some(keys), Some(buttons)] => Some(keys.is_empty() && buttons.is_empty()),
        _ => None,
    };
    let holds_ok = match holds_after {
        None | Some(Value::Null) => true,
        Some(v) => v.as_array().is_some_and(|a| a.is_empty()),
    };
    let check = Check::new(
        "nothing_held",
        Source::Bridge,
        held_ok.map(|h| h && holds_ok),
        true,
    )
    .with_after(json!({ "held": held_after, "holds": holds_after }));
    Evidence::judge(vec![check])
}

#[cfg(test)]
#[path = "input_proof_tests.rs"]
mod tests;

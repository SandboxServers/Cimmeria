//! Frame freshness of post-reads (C4).
//!
//! A post-read taken right after an action can still show the world from
//! before the action: the client applies input on a later frame. Comparing
//! the bridge frame counter at the action with the one at the read tells a
//! fresh read from a stale one.

use serde_json::{json, Value};

/// Frames a read must trail the action by to count as fresh.
pub const FRESH_FRAMES: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    /// Either frame is unknown (an old DLL, or no frame source).
    Unstamped,
}

/// Fresh when `read >= action + FRESH_FRAMES`; stale when both are known
/// and it is not; unstamped when either is missing.
pub fn freshness(action_frame: Option<u64>, read_frame: Option<u64>) -> Freshness {
    match (action_frame, read_frame) {
        (Some(a), Some(r)) if r >= a.saturating_add(FRESH_FRAMES) => Freshness::Fresh,
        (Some(_), Some(_)) => Freshness::Stale,
        _ => Freshness::Unstamped,
    }
}

/// Mark a check's `after` as read without a frame stamp: an object gains
/// `unstamped: true`; anything else is wrapped as `{"value": v, "unstamped": true}`.
pub fn mark_unstamped(after: Value) -> Value {
    match after {
        Value::Object(mut m) => {
            m.insert("unstamped".into(), Value::Bool(true));
            Value::Object(m)
        }
        v => json!({ "value": v, "unstamped": true }),
    }
}

#[cfg(test)]
#[path = "fresh_tests.rs"]
mod tests;

//! Mode and hover transitions (`client.input.mode`, `client.input.hover`).
//!
//! One row per change of the controller's mode word or hover id. A value that
//! changes faster than the cap is held back and reported once the window
//! opens again, with the count of changes that were not emitted.

use std::sync::{Mutex, MutexGuard};

use serde_json::json;

use super::controller::{MODE_AUTORUN, MODE_JUMP, MODE_MOUSELOOK, MODE_MOUSEWALK};
use super::frame::NavFrame;
use crate::hooks::entity_trace::Fields;

/// At most this many `client.input.mode` rows a second.
pub const MODE_CAP_PER_SEC: u32 = 8;
/// At most this many `client.input.hover` rows a second.
pub const HOVER_CAP_PER_SEC: u32 = 4;
/// The length of the rate-limit window, in milliseconds.
pub const WINDOW_MS: u64 = 1000;

/// One emitted change: the new value, the value it replaced, and how many
/// changes were held back since the last emitted one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change<T> {
    /// The value now.
    pub value: T,
    /// The value last emitted before this one.
    pub prev: T,
    /// Changes not emitted since the last row.
    pub suppressed: u32,
}

/// Reports changes of a value, at most `cap` a window of [`WINDOW_MS`].
#[derive(Debug)]
pub struct RateLimitedDiff<T> {
    cap: u32,
    initialized: bool,
    last_seen: T,
    last_emitted: T,
    window_start_ms: u64,
    emitted_in_window: u32,
    suppressed: u32,
}

impl<T: Copy + PartialEq> RateLimitedDiff<T> {
    /// A diff that has seen no value yet. `zero` seeds both last values. `const`,
    /// so it can seed a `static`.
    pub const fn new(cap: u32, zero: T) -> Self {
        Self {
            cap,
            initialized: false,
            last_seen: zero,
            last_emitted: zero,
            window_start_ms: 0,
            emitted_in_window: 0,
            suppressed: 0,
        }
    }

    /// Feeds `value` seen at `now_ms`. The first value only sets the baseline.
    /// A change is emitted while the window has room; otherwise it is counted as
    /// suppressed and the first tick of the next window emits it.
    pub fn observe(&mut self, now_ms: u64, value: T) -> Option<Change<T>> {
        if !self.initialized {
            self.initialized = true;
            self.last_seen = value;
            self.last_emitted = value;
            self.window_start_ms = now_ms;
            return None;
        }
        if now_ms.saturating_sub(self.window_start_ms) >= WINDOW_MS {
            self.window_start_ms = now_ms;
            self.emitted_in_window = 0;
        }
        let changed = value != self.last_seen;
        self.last_seen = value;
        if value != self.last_emitted && self.emitted_in_window < self.cap {
            let c = Change {
                value,
                prev: self.last_emitted,
                suppressed: self.suppressed,
            };
            self.last_emitted = value;
            self.emitted_in_window += 1;
            self.suppressed = 0;
            return Some(c);
        }
        if changed {
            self.suppressed = self.suppressed.saturating_add(1);
        }
        None
    }
}

/// The fields of one `client.input.mode` row: the mode words as bools, and the
/// raw `mode` and `prev`.
pub fn mode_fields(c: &Change<u32>) -> Fields {
    let v = c.value;
    vec![
        ("mode", json!(v)),
        ("prev", json!(c.prev)),
        ("mouselook", json!(v & MODE_MOUSELOOK != 0)),
        ("jump", json!(v & MODE_JUMP != 0)),
        ("autorun", json!(v & MODE_AUTORUN != 0)),
        ("mousewalk", json!(v & MODE_MOUSEWALK != 0)),
        ("suppressed", json!(c.suppressed)),
    ]
}

/// The fields of one `client.input.hover` row. `entity_id` is `0` for no hover.
pub fn hover_fields(c: &Change<i32>) -> Fields {
    vec![
        ("entity_id", json!(c.value)),
        ("prev", json!(c.prev)),
        ("suppressed", json!(c.suppressed)),
    ]
}

/// The last mode word seen and emitted.
static MODE: Mutex<RateLimitedDiff<u32>> = Mutex::new(RateLimitedDiff::new(MODE_CAP_PER_SEC, 0));
/// The last hover id seen and emitted.
static HOVER: Mutex<RateLimitedDiff<i32>> = Mutex::new(RateLimitedDiff::new(HOVER_CAP_PER_SEC, 0));

/// Locks `m`, recovering a poisoned lock: a panic in another tick must not stop the poll.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs once per tick: emits a mode row and a hover row when either changed and
/// the window has room. Each lock is released before its row is emitted.
pub(super) fn step(frame: &NavFrame) {
    let mode = lock(&MODE).observe(frame.now_ms, frame.ctrl.mode);
    if let Some(c) = mode {
        crate::hooks::emit::emit("client.input.mode", "info", mode_fields(&c));
    }
    let hover = lock(&HOVER).observe(frame.now_ms, frame.ctrl.hover_id);
    if let Some(c) = hover {
        crate::hooks::emit::emit("client.input.hover", "info", hover_fields(&c));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The value of the field `key` in `f`, if present.
    fn value_of(f: &Fields, key: &str) -> Option<serde_json::Value> {
        f.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn first_value_is_silent() {
        let mut d = RateLimitedDiff::new(4, 0);
        assert_eq!(d.observe(0, 7), None);
    }

    #[test]
    fn one_change_emits_with_prev() {
        let mut d = RateLimitedDiff::new(4, 0);
        d.observe(0, 0);
        assert_eq!(
            d.observe(100, 5),
            Some(Change {
                value: 5,
                prev: 0,
                suppressed: 0
            })
        );
    }

    #[test]
    fn cap_then_catch_up() {
        let mut d = RateLimitedDiff::new(HOVER_CAP_PER_SEC, 0);
        d.observe(0, 0);
        let mut emitted = Vec::new();
        for i in 1..=10 {
            if let Some(c) = d.observe(i as u64 * 50, i) {
                emitted.push(c.value);
            }
        }
        assert_eq!(emitted, vec![1, 2, 3, 4]);
        assert_eq!(
            d.observe(1000, 10),
            Some(Change {
                value: 10,
                prev: 4,
                suppressed: 6
            })
        );
    }

    #[test]
    fn flip_and_back_emits_two() {
        let mut d = RateLimitedDiff::new(4, 0);
        d.observe(0, 0);
        assert_eq!(d.observe(100, 5).map(|c| c.prev), Some(0));
        let back = d.observe(200, 0);
        assert_eq!(back.map(|c| (c.value, c.prev)), Some((0, 5)));
    }

    #[test]
    fn unchanged_never_emits() {
        let mut d = RateLimitedDiff::new(4, 0);
        d.observe(0, 3);
        assert_eq!(d.observe(100, 3), None);
        assert_eq!(d.observe(5000, 3), None);
    }

    #[test]
    fn mode_fields_decodes_bits() {
        let f = mode_fields(&Change {
            value: 0x9,
            prev: 0,
            suppressed: 0,
        });
        assert_eq!(value_of(&f, "mouselook"), Some(json!(true)));
        assert_eq!(value_of(&f, "mousewalk"), Some(json!(true)));
        assert_eq!(value_of(&f, "jump"), Some(json!(false)));
        assert_eq!(value_of(&f, "autorun"), Some(json!(false)));
        assert_eq!(value_of(&f, "mode"), Some(json!(9)));
    }

    #[test]
    fn hover_fields_names() {
        let f = hover_fields(&Change {
            value: 42,
            prev: 0,
            suppressed: 1,
        });
        let keys: Vec<&str> = f.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec!["entity_id", "prev", "suppressed"]);
    }
}

//! Ability press and send tracing: the portable half of the
//! `client.ability.*` events (AB-C1 and AB-C2 of the ability-mechanics
//! telemetry plan).
//!
//! The anchors are in
//! `docs/reverse-engineering/findings/ability-client-hook-anchors.md`; the
//! detours that drive this module are in `hooks::inline_hooks::ability`.
//! Everything here is pure or reads memory through [`Mem`], so it is
//! unit-tested off the DLL target.
//!
//! | Target | When | Key fields |
//! |---|---|---|
//! | `client.ability.press` | The press chain knows which ability (or that the slot was empty, or the Lua call was malformed) | `press_id`, `source`, `slot`, `ability_id`, `target_id` |
//! | `client.ability.press_dropped` | The client discarded the press, or refused an allowlisted send, without a wire message | `press_id`, `reason`, `drop_site` |
//! | `client.ability.sent` | `RouteOutgoingEntityRpc` reached a `start*Message` for an allowlisted method | `send_id`, `press_id`, `method`, decoded arguments, `client_target_id` |
//! | `client.ability.sent_seq` | The network thread stamped the packets of the bundle that carried the call | `send_id`, `mercury_seq_first`, `mercury_seq_last` |
//!
//! [`Mem`]: crate::hooks::entity_trace::map::Mem

pub(crate) mod decode;
pub(crate) mod layout;
pub(crate) mod press;
pub(crate) mod seq_join;

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use super::entity_trace::Fields;
use super::name_throttle::{Decision, NameThrottle};

/// A press the client saw, once its ability (or its failure) is known.
pub(crate) const TARGET_PRESS: &str = "client.ability.press";
/// A press or an allowlisted send the client discarded.
pub(crate) const TARGET_DROPPED: &str = "client.ability.press_dropped";
/// An allowlisted method the router sent.
pub(crate) const TARGET_SENT: &str = "client.ability.sent";
/// The Mercury sequence range of the bundle that carried a sent method.
pub(crate) const TARGET_SENT_SEQ: &str = "client.ability.sent_seq";

/// One event ready to go: target, level, throttle key and fields.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Out {
    /// The `client.ability.*` target.
    pub target: &'static str,
    /// `info` or `debug`.
    pub level: &'static str,
    /// The per-name throttle key (D-AU5: per name, not per entity).
    pub key: String,
    /// Event fields, in emit order.
    pub fields: Fields,
}

/// The value of `name` in `fields`, for tests and joins.
#[cfg(test)]
pub(crate) fn field<'a>(fields: &'a Fields, name: &str) -> Option<&'a serde_json::Value> {
    fields.iter().find(|(k, _)| *k == name).map(|(_, v)| v)
}

// ---------------------------------------------------------------------
// Volume: D-AU5, a burst of 8 then 4 a second per name, with the count of
// suppressed events on the next one through.

/// The per-name token buckets for every `client.ability.*` target.
#[derive(Debug, Default)]
pub(crate) struct AbilityThrottle {
    table: NameThrottle,
}

impl AbilityThrottle {
    /// Decide for one event under `key` at monotonic `now_ms`; `None` when
    /// suppressed, else the fields with `suppressed` added when non-zero.
    pub(crate) fn admit(&mut self, key: &str, mut fields: Fields, now_ms: u64) -> Option<Fields> {
        let Decision::Emit { suppressed } = self.table.check(key, now_ms) else {
            return None;
        };
        if suppressed > 0 {
            fields.push(("suppressed", serde_json::json!(suppressed)));
        }
        Some(fields)
    }
}

static THROTTLE: Mutex<Option<AbilityThrottle>> = Mutex::new(None);
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// Milliseconds since the first ability event of the process.
pub(crate) fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Send `outs` through the throttle to the uploader and the lab ring.
/// Under `cfg(test)` every event is also recorded, before the throttle, in
/// a per-thread capture that the detour tests read.
pub(crate) fn report(outs: Vec<Out>) {
    for out in outs {
        #[cfg(test)]
        capture::record(&out);
        let admitted = {
            let mut g = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
            g.get_or_insert_with(AbilityThrottle::default)
                .admit(&out.key, out.fields, now_ms())
        };
        #[cfg(all(target_os = "windows", target_arch = "x86"))]
        if let Some(fields) = admitted {
            crate::hooks::emit::emit(out.target, out.level, fields);
        }
        #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
        let _ = admitted;
    }
}

/// Per-thread record of every event [`report`] saw (tests only).
#[cfg(test)]
pub(crate) mod capture {
    use super::Out;
    use std::cell::RefCell;

    thread_local! {
        static SEEN: RefCell<Vec<Out>> = const { RefCell::new(Vec::new()) };
    }

    pub(crate) fn record(out: &Out) {
        SEEN.with(|s| s.borrow_mut().push(out.clone()));
    }

    /// Take everything recorded on this thread so far.
    pub(crate) fn take() -> Vec<Out> {
        SEEN.with(|s| std::mem::take(&mut *s.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row() -> Fields {
        vec![("press_id", json!(1))]
    }

    /// D-AU5: eight back to back, then four a second, and the next event
    /// through carries how many were dropped.
    #[test]
    fn the_throttle_is_a_burst_of_eight_then_four_a_second() {
        let mut t = AbilityThrottle::default();
        for i in 0..8 {
            assert!(t.admit("press:hotbar", row(), 0).is_some(), "burst {i}");
        }
        for _ in 0..5 {
            assert!(t.admit("press:hotbar", row(), 0).is_none());
        }
        let next = t
            .admit("press:hotbar", row(), 250)
            .expect("one token refilled");
        assert_eq!(field(&next, "suppressed"), Some(&json!(5)));
        assert!(t.admit("press:hotbar", row(), 250).is_none());
    }

    /// A storm on one name never silences another.
    #[test]
    fn one_hot_name_does_not_hide_another() {
        let mut t = AbilityThrottle::default();
        for _ in 0..100 {
            t.admit("sent:useAbility", row(), 0);
        }
        let other = t.admit("sent:confirmationResponse", row(), 0);
        assert!(other.is_some());
        assert_eq!(field(&other.unwrap(), "suppressed"), None);
    }
}

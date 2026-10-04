//! The client's half of an ability cast, the portable part of the
//! `client.ability.*` events (Part 2 of
//! `docs/analysis/ability-mechanics/lab-uat-and-telemetry.md`): the press
//! and what the client sent (AB-C1, AB-C2: [`press`], [`decode`],
//! [`seq_join`]), what it received, decoded (AB-C3: [`recv`],
//! [`recv_methods`], [`wire_decode`]), what it applied (AB-C4: [`applied`],
//! [`event_bag`], [`clock`]) and what it asked its UI to show (AB-C5:
//! [`shown`]).
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
//! | `client.ability.recv` | An inbound ability method, decoded at `onEntityMethod` before the game reads it | `method`, `entity_id`, the arguments by name, `cast_id` |
//! | `client.ability.applied` | A stock handler applied it: effect bar, cooldown, stat, state flag | `kind`, `entity_id`, kind-specific fields |
//! | `client.ability.shown` | A UI handler ran for it (combat text, chat line, effect bar) or a sequence played | `kind`, `handler`, `status`, `cast_id` |
//!
//! # Volume (D-AU5)
//!
//! One table ([`throttle::AbilityThrottle`]) holds every ability bucket:
//! burst 8, then 4 a second per name, and the next event of that name
//! carries the dropped count as `suppressed`. Two rules sit on it:
//!
//! - A press and its answer are kept or dropped together (AB-C2): rows
//!   with a `press_id` follow the decision made for their press row.
//! - Recv, applied and shown rows go through their own per-name bucket
//!   ([`admit`]), keyed by method or kind and by [`whose`] (`self` for
//!   the local player, `other` for any other being), so a stat storm
//!   cannot hide an `onEffectResults` and a fight's NPC traffic cannot
//!   starve the player's own rows.
//!
//! The governor forwards `client.ability.*` untouched
//! (`KeepReason::AbilityTrace`): the hook already budgets it, and a second
//! budget would drop events the `suppressed` counts do not know about.
//!
//! [`Mem`]: crate::hooks::entity_trace::map::Mem

pub(crate) mod applied;
pub(crate) mod clock;
#[cfg(test)]
pub(crate) mod coverage;
pub(crate) mod decode;
pub(crate) mod event_bag;
pub(crate) mod layout;
pub(crate) mod press;
pub(crate) mod recv;
pub(crate) mod recv_methods;
pub(crate) mod seq_join;
pub(crate) mod shown;
pub(crate) mod throttle;
pub(crate) mod timing;
pub(crate) mod wire_decode;

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde_json::json;

use super::entity_trace::Fields;
use super::name_throttle::Decision;
use throttle::AbilityThrottle;

/// A press the client saw, once its ability (or its failure) is known.
pub(crate) const TARGET_PRESS: &str = "client.ability.press";
/// A press or an allowlisted send the client discarded.
pub(crate) const TARGET_DROPPED: &str = "client.ability.press_dropped";
/// An allowlisted method the router sent.
pub(crate) const TARGET_SENT: &str = "client.ability.sent";
/// The Mercury sequence range of the bundle that carried a sent method.
pub(crate) const TARGET_SENT_SEQ: &str = "client.ability.sent_seq";
/// An inbound ability method, decoded.
pub(crate) const TARGET_RECV: &str = "client.ability.recv";

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

static THROTTLE: Mutex<Option<AbilityThrottle>> = Mutex::new(None);
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// Milliseconds since the first ability event of the process.
pub(crate) fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// The last part of a recv / applied / shown throttle key: `self` for the
/// local player, `other` for anyone else (or unknown).
pub(crate) fn whose(local: bool) -> &'static str {
    if local {
        "self"
    } else {
        "other"
    }
}

/// One per-name bucket decision for `key` in the shared table. Poisoning
/// is ignored.
pub(crate) fn throttle(key: &str) -> Decision {
    let mut g = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(AbilityThrottle::default)
        .check_name(key, now_ms())
}

/// For a recv / applied / shown row: `fields` with the bucket's
/// `suppressed` count, or `None` when the bucket for `key` is empty.
/// `build` runs only when the event goes out, so a suppressed event costs
/// no decoding. These rows carry no `press_id`, so the press rule does not
/// apply to them.
pub(crate) fn admit(key: &str, build: impl FnOnce() -> Fields) -> Option<Fields> {
    let Decision::Emit { suppressed } = throttle(key) else {
        return None;
    };
    let mut f = build();
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    Some(f)
}

/// Send `outs` through the throttle to the uploader and the lab ring.
/// Under `cfg(test)` every event is also recorded, before the throttle, in
/// a per-thread capture that the detour tests read.
pub(crate) fn report(outs: Vec<Out>) {
    for out in outs {
        #[cfg(test)]
        capture::record(&out);
        let (target, level) = (out.target, out.level);
        let admitted = {
            let mut g = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
            g.get_or_insert_with(AbilityThrottle::default)
                .admit(out, now_ms())
        };
        #[cfg(all(target_os = "windows", target_arch = "x86"))]
        if let Some(fields) = admitted {
            crate::hooks::emit::emit(target, level, fields);
        }
        #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
        let _ = (target, level);
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

    /// A hot name is held to the bucket; a different name still gets
    /// through during the burst, and the next emitted event of the hot
    /// name carries the count it lost.
    #[test]
    fn the_bucket_is_per_name_and_counts_what_it_drops() {
        let mut emitted = 0;
        let mut builds = 0;
        for _ in 0..200 {
            if admit("test:hot", || {
                builds += 1;
                vec![]
            })
            .is_some()
            {
                emitted += 1;
            }
        }
        assert!((8..=12).contains(&emitted), "{emitted}");
        assert_eq!(builds, emitted, "a suppressed event is never built");
        assert!(admit("test:rare", Vec::new).is_some());
    }

    #[test]
    fn the_suppressed_count_rides_the_next_emitted_event() {
        for _ in 0..50 {
            let _ = admit("test:count", Vec::new);
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        let f = admit("test:count", Vec::new).expect("the bucket refilled");
        let suppressed = f
            .iter()
            .find(|(k, _)| *k == "suppressed")
            .map(|(_, v)| v.as_u64().unwrap());
        assert!(suppressed.is_some_and(|n| n >= 40), "{suppressed:?}");
    }
}

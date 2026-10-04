//! The client's half of an ability cast: what it received, decoded
//! (`client.ability.recv`, AB-C3 of the ability-mechanics telemetry plan),
//! what it applied (`client.ability.applied`, AB-C4: [`applied`]) and what
//! it asked its UI to show (`client.ability.shown`, AB-C5: [`shown`]).
//!
//! Plan: `docs/analysis/ability-mechanics/lab-uat-and-telemetry.md` Part 2.
//! Anchors: `docs/reverse-engineering/findings/ability-client-hook-anchors.md`.
//!
//! Everything here is portable and unit-tested off the DLL target: the
//! wire decoder ([`wire_decode`]), the method table it is driven by
//! ([`recv_methods`]), and the glue that turns one inbound entity method
//! into an event ([`recv`]). The detour that feeds it is the existing
//! `EntityManager::onEntityMethod` hook
//! (`hooks::inline_hooks::entity_messages`), which reads the message's
//! argument bytes before the original consumes them.
//!
//! # Volume (D-AU5)
//!
//! Every event goes through one per-name token bucket: burst 8, then 4 a
//! second, and the next event of that name carries the dropped count as
//! `suppressed`. The name is the event's key (`recv:onEffectResults:self`),
//! so a stat storm cannot hide an `onEffectResults`, and the key's last
//! part ([`whose`]) keeps the local player's rows apart from every other
//! being's, so a fight's NPC traffic cannot starve the player's own. The governor forwards
//! `client.ability.*` untouched (`governor::classify`, `SourceThrottled`):
//! the throttle here is the budget, and a second one in the governor would
//! drop what this one already counted.

pub(crate) mod applied;
pub(crate) mod clock;
pub(crate) mod event_bag;
pub(crate) mod recv;
pub(crate) mod recv_methods;
pub(crate) mod shown;
pub(crate) mod wire_decode;

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde_json::json;

use super::entity_trace::Fields;
use super::name_throttle::{Decision, NameThrottle};

/// `client.ability.recv`: an inbound ability method, decoded.
pub(crate) const TARGET_RECV: &str = "client.ability.recv";

static THROTTLE: Mutex<Option<NameThrottle>> = Mutex::new(None);
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// The last part of a throttle key: `self` for the local player, `other`
/// for anyone else (or unknown).
pub(crate) fn whose(local: bool) -> &'static str {
    if local {
        "self"
    } else {
        "other"
    }
}

/// Run the shared per-name bucket for `key`. Poisoning is ignored.
pub(crate) fn throttle(key: &str) -> Decision {
    let now_ms = EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64;
    let mut g = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(NameThrottle::new).check(key, now_ms)
}

/// `fields` with the bucket's `suppressed` count, or `None` when the
/// bucket for `key` is empty. `build` runs only when the event goes out,
/// so a suppressed event costs no decoding.
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

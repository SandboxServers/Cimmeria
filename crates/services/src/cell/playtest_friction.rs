//! Stuck-player detectors — telemetry for "something should have happened and
//! did not", raised from the player's *behaviour* rather than from knowing the
//! cause.
//!
//! Every signal here was visible in hindsight in the 2026-09-18 colo playtest
//! (`docs/analysis/playtests/2026-09-18-colo-castle/README.md` §9.3): 76 clicks
//! on a DHD with no handler, an item used repeatedly against a step that was
//! not active yet, a tester hunting for a console command that does not exist,
//! an escort 95 units behind its leader. None of them produced a warning.
//!
//! One event shape, target `playtest.friction`, WARN, discriminated by
//! `signal`. A detector fires **once per episode**: when the same
//! `(actor, subject)` pair repeats `threshold` times inside `window`. A
//! condition that persists re-fires once per window, so volume is bounded by
//! construction.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Interacts on one target that all dead-ended.
pub(crate) const INTERACT_THRESHOLD: u32 = 5;
pub(crate) const INTERACT_WINDOW: Duration = Duration::from_secs(60);
/// Uses of one item that matched no content chain.
pub(crate) const ITEM_USE_THRESHOLD: u32 = 2;
pub(crate) const ITEM_USE_WINDOW: Duration = Duration::from_secs(120);
/// Rejected `.`-console lines from one GM.
pub(crate) const CONSOLE_REJECT_THRESHOLD: u32 = 3;
pub(crate) const CONSOLE_REJECT_WINDOW: Duration = Duration::from_secs(120);
/// Consecutive AI ticks (2 s each) an escort spent far outside its band.
pub(crate) const ESCORT_THRESHOLD: u32 = 5;
pub(crate) const ESCORT_WINDOW: Duration = Duration::from_secs(60);
/// "Far outside" = this multiple of `follow_max_distance`.
pub(crate) const ESCORT_SEPARATION_FACTOR: f32 = 3.0;

const PRUNE_AT: usize = 2048;

/// Counts repeats of a `(actor, subject)` key inside a sliding episode window.
#[derive(Debug, Default)]
pub(crate) struct EpisodeCounter {
    entries: HashMap<(u32, u64), (u32, Instant)>,
}

impl EpisodeCounter {
    /// Record one occurrence. Returns `Some(count)` exactly once per episode —
    /// on the occurrence that reaches `threshold` — and `None` otherwise.
    pub(crate) fn note(
        &mut self,
        key: (u32, u64),
        threshold: u32,
        window: Duration,
        now: Instant,
    ) -> Option<u32> {
        if self.entries.len() >= PRUNE_AT {
            self.entries
                .retain(|_, (_, first)| now.saturating_duration_since(*first) <= window);
        }
        let entry = self.entries.entry(key).or_insert((0, now));
        if now.saturating_duration_since(entry.1) > window {
            *entry = (0, now);
        }
        entry.0 += 1;
        (entry.0 == threshold).then_some(entry.0)
    }

    /// Forget a key — the condition cleared (escort back in band, etc.).
    pub(crate) fn clear(&mut self, key: (u32, u64)) {
        self.entries.remove(&key);
    }
}

#[derive(Default)]
struct Detectors {
    interact: EpisodeCounter,
    item_use: EpisodeCounter,
    console: EpisodeCounter,
    escort: EpisodeCounter,
}

static DETECTORS: LazyLock<Mutex<Detectors>> = LazyLock::new(Mutex::default);

fn with<R>(f: impl FnOnce(&mut Detectors) -> R) -> R {
    // A poisoned lock only means another thread panicked mid-count; the
    // counters are still usable and telemetry must never take the cell down.
    let mut guard = DETECTORS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut guard)
}

/// An interact reached the end of the dispatcher with nothing to do.
pub(crate) fn interact_no_effect(
    entity_id: u32,
    target_entity_id: u32,
    npc_name: &str,
    tag: &str,
    interaction_flags: i64,
) {
    let fired = with(|d| {
        d.interact.note(
            (entity_id, u64::from(target_entity_id)),
            INTERACT_THRESHOLD,
            INTERACT_WINDOW,
            Instant::now(),
        )
    });
    if let Some(count) = fired {
        tracing::warn!(
            target: "playtest.friction",
            signal = "repeat_interact_no_effect",
            reason = "interact_dead_end",
            entity_id,
            target_entity_id,
            npc_name,
            tag,
            interaction_flags,
            count,
            window_secs = INTERACT_WINDOW.as_secs(),
            "friction: player keeps interacting with a target and nothing happens -- likely a missing handler or an inert dialog set"
        );
    }
}

/// An item use matched no content chain.
pub(crate) fn item_use_no_chain(entity_id: u32, item_id: i32) {
    let fired = with(|d| {
        d.item_use.note(
            (entity_id, item_id as u32 as u64),
            ITEM_USE_THRESHOLD,
            ITEM_USE_WINDOW,
            Instant::now(),
        )
    });
    if let Some(count) = fired {
        tracing::warn!(
            target: "playtest.friction",
            signal = "repeat_item_use_no_chain",
            reason = "no_chain_matched",
            entity_id,
            item_id,
            count,
            window_secs = ITEM_USE_WINDOW.as_secs(),
            "friction: player keeps using an item that matches no content chain -- likely a mission step that has not activated or a condition that never holds"
        );
    }
}

/// A `.`-console line was rejected before dispatch.
pub(crate) fn console_rejected(entity_id: u32, command: &str, reject: &str) {
    tracing::debug!(
        entity_id,
        command,
        reason = reject,
        "GM .-console command rejected -- feedback sent to client"
    );
    let fired = with(|d| {
        d.console.note(
            (entity_id, 0),
            CONSOLE_REJECT_THRESHOLD,
            CONSOLE_REJECT_WINDOW,
            Instant::now(),
        )
    });
    if let Some(count) = fired {
        tracing::warn!(
            target: "playtest.friction",
            signal = "console_reject_streak",
            reason = reject,
            entity_id,
            command,
            count,
            window_secs = CONSOLE_REJECT_WINDOW.as_secs(),
            "friction: GM keeps sending rejected console commands -- they are hunting for a command that does not exist or has a different shape"
        );
    }
}

/// An escort's follow tick. `dist` is the current distance to its leader.
pub(crate) fn escort_tick(npc_id: u32, target_id: u32, dist: f32, max_d: f32, routed: bool) {
    let key = (npc_id, u64::from(target_id));
    if dist <= max_d * ESCORT_SEPARATION_FACTOR {
        with(|d| d.escort.clear(key));
        return;
    }
    let fired = with(|d| {
        d.escort
            .note(key, ESCORT_THRESHOLD, ESCORT_WINDOW, Instant::now())
    });
    if let Some(count) = fired {
        tracing::warn!(
            target: "playtest.friction",
            signal = "escort_separated",
            reason = if routed { "cannot_keep_up" } else { "unrouted" },
            npc_id,
            target_id,
            dist,
            max_d,
            count,
            "friction: escort is far outside its follow band and not closing -- too slow, unrouted, or stuck"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn episode_fires_exactly_once_at_threshold() {
        let mut c = EpisodeCounter::default();
        let t0 = Instant::now();
        let w = Duration::from_secs(60);
        let fired: Vec<Option<u32>> = (0..8)
            .map(|i| c.note((1, 9), 5, w, t0 + Duration::from_secs(i)))
            .collect();
        assert_eq!(fired.iter().filter(|f| f.is_some()).count(), 1);
        assert_eq!(fired[4], Some(5), "fires on the 5th repeat, not before");
    }

    #[test]
    fn episode_resets_after_the_window_and_is_per_key() {
        let mut c = EpisodeCounter::default();
        let t0 = Instant::now();
        let w = Duration::from_secs(60);
        for i in 0..4 {
            assert_eq!(c.note((1, 9), 5, w, t0 + Duration::from_secs(i)), None);
        }
        // A different subject does not inherit the count.
        assert_eq!(c.note((1, 10), 5, w, t0), None);
        // Past the window the 5th occurrence starts a NEW episode at 1.
        assert_eq!(c.note((1, 9), 5, w, t0 + Duration::from_secs(120)), None);
        // A persisting condition re-fires once per window.
        let later = t0 + Duration::from_secs(121);
        let refired = (0..4).filter_map(|_| c.note((1, 9), 5, w, later)).count();
        assert_eq!(refired, 1);
    }

    #[test]
    fn clear_forgets_progress() {
        let mut c = EpisodeCounter::default();
        let t0 = Instant::now();
        let w = Duration::from_secs(60);
        for _ in 0..4 {
            c.note((7, 7), 5, w, t0);
        }
        c.clear((7, 7));
        assert_eq!(c.note((7, 7), 5, w, t0), None, "count restarted at 1");
    }

    /// The DHD shape: 76 dead-end clicks must produce one warn, not zero and
    /// not 76.
    #[test]
    fn repeat_dead_end_interacts_warn_once() {
        let logs = crate::test_support::LogCapture::install();
        for _ in 0..20 {
            interact_no_effect(4_000_001, 4_000_002, "DHD_Frost", "Castle_DHD", 16);
        }
        let ev = logs
            .find_event(
                tracing::Level::WARN,
                "keeps interacting with a target",
                "interact_dead_end",
            )
            .expect("friction warn");
        assert_eq!(ev.target, "playtest.friction");
        assert!(ev.has_field("signal", "repeat_interact_no_effect"));
        assert!(ev.has_field("interaction_flags", "16"));
        assert!(ev.has_field("count", "5"));
    }
}

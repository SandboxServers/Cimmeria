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
//! Signals: `repeat_interact_no_effect`, `repeat_item_use_no_chain`,
//! `console_reject_streak`, `escort_separated` (episode counters, below) and
//! `step_stalled`, `region_dwell_no_hint`, `death_then_silence`,
//! `dialog_displaced`, `objective_never_completed` ([`PlayerWatch`]).
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

// ── Per-player watch: signals that need state across time ──────────────────

/// How often [`player_tick`] re-evaluates one player.
pub(crate) const WATCH_EVAL_INTERVAL: Duration = Duration::from_secs(2);
/// A mission step this old, on a player who is still sending movement, is
/// reported once.
pub(crate) const STEP_STALL_AFTER: Duration = Duration::from_secs(300);
/// Server-side containment must hold this long with no client hint.
pub(crate) const REGION_DWELL_AFTER: Duration = Duration::from_secs(6);
/// A hint this recent counts for a region the server only now sees entered
/// (the client detects the edge before our 2 s evaluation does).
pub(crate) const REGION_HINT_GRACE: Duration = Duration::from_secs(30);
/// A second dialog this soon after the first replaces it before it can be read.
pub(crate) const DIALOG_DISPLACED_WITHIN: Duration = Duration::from_secs(3);
/// Post-respawn silence: no region hint after this long...
pub(crate) const RESPAWN_SILENCE_AFTER: Duration = Duration::from_secs(120);
/// ...and this much travel, for a client that was hinting before it died.
pub(crate) const RESPAWN_SILENCE_MIN_TRAVEL: f32 = 100.0;
pub(crate) const RESPAWN_SILENCE_MIN_PRIOR_HINTS: u32 = 3;
/// Position deltas above this between two evaluations are teleports, not travel.
const TELEPORT_JUMP: f32 = 50.0;

/// One detected condition. Pure data so the detector logic is testable
/// without a tracing subscriber.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Friction {
    StepStalled {
        mission_id: i32,
        step_id: i32,
        age_secs: u64,
    },
    RegionDwellNoHint {
        region_id: u32,
        region_tag: String,
        dwell_secs: u64,
    },
    DeathThenSilence {
        secs_since_respawn: u64,
        travelled: f32,
        hints_before_respawn: u32,
    },
    DialogDisplaced {
        dialog_id: i32,
        replaced_dialog_id: i32,
        ms_since_previous: u64,
    },
}

#[derive(Debug, Default)]
pub(crate) struct PlayerWatch {
    last_eval: Option<Instant>,
    last_pos: Option<[f32; 3]>,
    /// mission_id -> (step_id, first seen, reported)
    steps: HashMap<i32, (i32, Instant, bool)>,
    /// region runtime id -> (server-side entry time, reported)
    regions: HashMap<u32, (Instant, bool)>,
    /// region runtime id -> last client hint
    hints: HashMap<u32, Instant>,
    hints_total: u32,
    respawned_at: Option<Instant>,
    hints_before_respawn: u32,
    hints_since_respawn: u32,
    travelled_since_respawn: f32,
    silence_reported: bool,
    last_dialog: Option<(i32, Instant)>,
}

impl PlayerWatch {
    pub(crate) fn note_region_hint(&mut self, region_id: u32, now: Instant) {
        self.hints.insert(region_id, now);
        self.hints_total += 1;
        self.hints_since_respawn += 1;
    }

    pub(crate) fn note_respawn(&mut self, now: Instant) {
        self.respawned_at = Some(now);
        self.hints_before_respawn = self.hints_total;
        self.hints_since_respawn = 0;
        self.travelled_since_respawn = 0.0;
        self.silence_reported = false;
    }

    pub(crate) fn note_dialog(&mut self, dialog_id: i32, now: Instant) -> Option<Friction> {
        let out = match self.last_dialog {
            Some((prev, at))
                if prev != dialog_id
                    && now.saturating_duration_since(at) < DIALOG_DISPLACED_WITHIN =>
            {
                Some(Friction::DialogDisplaced {
                    dialog_id,
                    replaced_dialog_id: prev,
                    ms_since_previous: now.saturating_duration_since(at).as_millis() as u64,
                })
            }
            _ => None,
        };
        self.last_dialog = Some((dialog_id, now));
        out
    }

    /// Re-evaluate the time-based signals. `missions` is
    /// `(mission_id, current_step_id)` for every active, non-hidden mission;
    /// `regions_inside` is every region the server believes contains `pos`.
    pub(crate) fn evaluate(
        &mut self,
        now: Instant,
        pos: [f32; 3],
        missions: &[(i32, i32)],
        regions_inside: &[(u32, String)],
    ) -> Vec<Friction> {
        let mut out = Vec::new();

        if let Some(prev) = self.last_pos {
            let d = ((pos[0] - prev[0]).powi(2) + (pos[2] - prev[2]).powi(2)).sqrt();
            if d < TELEPORT_JUMP {
                self.travelled_since_respawn += d;
            }
        }
        self.last_pos = Some(pos);

        // step_stalled
        self.steps
            .retain(|mid, _| missions.iter().any(|(m, _)| m == mid));
        for &(mission_id, step_id) in missions {
            let e = self
                .steps
                .entry(mission_id)
                .or_insert((step_id, now, false));
            if e.0 != step_id {
                *e = (step_id, now, false);
            }
            let age = now.saturating_duration_since(e.1);
            if !e.2 && age >= STEP_STALL_AFTER {
                e.2 = true;
                out.push(Friction::StepStalled {
                    mission_id,
                    step_id,
                    age_secs: age.as_secs(),
                });
            }
        }

        // region_dwell_no_hint
        self.regions
            .retain(|rid, _| regions_inside.iter().any(|(r, _)| r == rid));
        for (region_id, tag) in regions_inside {
            let e = self.regions.entry(*region_id).or_insert((now, false));
            let entered = e.0;
            let hinted = self.hints.get(region_id).is_some_and(|h| {
                *h >= entered || entered.saturating_duration_since(*h) <= REGION_HINT_GRACE
            });
            let dwell = now.saturating_duration_since(entered);
            if !e.1 && !hinted && dwell >= REGION_DWELL_AFTER {
                e.1 = true;
                out.push(Friction::RegionDwellNoHint {
                    region_id: *region_id,
                    region_tag: tag.clone(),
                    dwell_secs: dwell.as_secs(),
                });
            }
        }

        // death_then_silence
        if let Some(at) = self.respawned_at {
            let since = now.saturating_duration_since(at);
            if !self.silence_reported
                && self.hints_since_respawn == 0
                && self.hints_before_respawn >= RESPAWN_SILENCE_MIN_PRIOR_HINTS
                && since >= RESPAWN_SILENCE_AFTER
                && self.travelled_since_respawn >= RESPAWN_SILENCE_MIN_TRAVEL
            {
                self.silence_reported = true;
                out.push(Friction::DeathThenSilence {
                    secs_since_respawn: since.as_secs(),
                    travelled: self.travelled_since_respawn,
                    hints_before_respawn: self.hints_before_respawn,
                });
            }
        }
        out
    }
}

/// XZ point-in-polygon (ray casting) against a region's `points`.
pub(crate) fn region_contains_xz(points: &[[f32; 3]], x: f32, z: f32) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (xi, zi) = (points[i][0], points[i][2]);
        let (xj, zj) = (points[j][0], points[j][2]);
        if (zi > z) != (zj > z) && x < (xj - xi) * (z - zi) / (zj - zi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

static WATCHES: LazyLock<Mutex<HashMap<u32, PlayerWatch>>> = LazyLock::new(Mutex::default);

fn with_watch<R>(entity_id: u32, f: impl FnOnce(&mut PlayerWatch) -> R) -> R {
    let mut guard = WATCHES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(guard.entry(entity_id).or_default())
}

fn emit(entity_id: u32, f: &Friction) {
    match f {
        Friction::StepStalled {
            mission_id,
            step_id,
            age_secs,
        } => tracing::warn!(
            target: "playtest.friction",
            signal = "step_stalled",
            reason = "step_not_advancing",
            entity_id,
            mission_id,
            step_id,
            age_secs,
            "friction: mission step has not advanced while the player keeps playing -- a trigger may not be firing (normal for long grind steps)"
        ),
        Friction::RegionDwellNoHint {
            region_id,
            region_tag,
            dwell_secs,
        } => tracing::warn!(
            target: "playtest.friction",
            signal = "region_dwell_no_hint",
            reason = "client_region_hint_missing",
            entity_id,
            region_id,
            region_tag = %region_tag,
            dwell_secs,
            "friction: server sees the player inside a region but the client sent no hint for it -- region chains will not fire"
        ),
        Friction::DeathThenSilence {
            secs_since_respawn,
            travelled,
            hints_before_respawn,
        } => tracing::warn!(
            target: "playtest.friction",
            signal = "death_then_silence",
            reason = "no_region_hints_since_respawn",
            entity_id,
            secs_since_respawn,
            travelled,
            hints_before_respawn,
            "friction: client was sending region hints before it died and has sent none since respawning -- region-triggered content is dead for this session"
        ),
        Friction::DialogDisplaced {
            dialog_id,
            replaced_dialog_id,
            ms_since_previous,
        } => tracing::warn!(
            target: "playtest.friction",
            signal = "dialog_displaced",
            reason = "dialog_replaced_too_fast",
            entity_id,
            dialog_id,
            replaced_dialog_id,
            ms_since_previous,
            "friction: a dialog was replaced before the player could read it"
        ),
    }
}

/// Called on every accepted player movement packet; self-throttles to
/// [`WATCH_EVAL_INTERVAL`] per player.
pub(crate) fn player_tick(
    space_mgr: &crate::cell::space_manager::SpaceManager,
    entity_id: u32,
    pos: [f32; 3],
) {
    let now = Instant::now();
    let due = with_watch(entity_id, |w| {
        let due = w
            .last_eval
            .is_none_or(|t| now.saturating_duration_since(t) >= WATCH_EVAL_INTERVAL);
        if due {
            w.last_eval = Some(now);
        }
        due
    });
    if !due {
        return;
    }
    let Some(e) = space_mgr.get_entity(entity_id) else {
        return;
    };
    let missions: Vec<(i32, i32)> = e
        .missions
        .active_missions()
        .into_iter()
        .filter(|m| !m.is_hidden)
        .filter_map(|m| m.current_step_id.map(|s| (m.mission_id, s)))
        .collect();
    let world = space_mgr
        .get_entity_world_name(entity_id)
        .unwrap_or_default();
    let regions: Vec<(u32, String)> = space_mgr
        .regions_for_world(&world)
        .into_iter()
        .filter(|r| region_contains_xz(&r.points, pos[0], pos[2]))
        .map(|r| (r.runtime_id, r.tag.clone()))
        .collect();
    let fired = with_watch(entity_id, |w| w.evaluate(now, pos, &missions, &regions));
    for f in &fired {
        emit(entity_id, f);
    }
}

/// The client reported a region edge (`triggerClientHintedGenericRegion`).
pub(crate) fn region_hint(entity_id: u32, region_id: u32) {
    with_watch(entity_id, |w| w.note_region_hint(region_id, Instant::now()));
}

/// The player respawned (`callForAid`).
pub(crate) fn respawned(entity_id: u32) {
    with_watch(entity_id, |w| w.note_respawn(Instant::now()));
}

/// A dialog is about to be displayed to the player.
pub(crate) fn dialog_shown(entity_id: u32, dialog_id: i32) {
    if let Some(f) = with_watch(entity_id, |w| w.note_dialog(dialog_id, Instant::now())) {
        emit(entity_id, &f);
    }
}

/// A mission is being completed by a chain while objectives are still open.
pub(crate) fn objectives_never_completed(entity_id: u32, mission_id: i32, open: &[(i32, bool)]) {
    for &(objective_id, optional) in open {
        tracing::warn!(
            target: "playtest.friction",
            signal = "objective_never_completed",
            reason = "objective_open_at_mission_complete",
            entity_id,
            mission_id,
            objective_id,
            optional,
            "friction: mission completed with an objective the player never completed -- its trigger may be unreachable"
        );
    }
}

/// Drop all per-entity state (entity ids are recycled).
pub(crate) fn forget(entity_id: u32) {
    WATCHES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&entity_id);
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

    fn secs(t0: Instant, s: u64) -> Instant {
        t0 + Duration::from_secs(s)
    }

    /// Throne Room shape: server-side containment, no client hint.
    #[test]
    fn region_dwell_fires_once_and_a_hint_suppresses_it() {
        let t0 = Instant::now();
        let inside = [(7u32, "Castle.ThroneRoom".to_string())];
        let mut w = PlayerWatch::default();
        assert!(w.evaluate(t0, [0.0; 3], &[], &inside).is_empty());
        let fired = w.evaluate(secs(t0, 6), [0.0; 3], &[], &inside);
        assert_eq!(
            fired,
            vec![Friction::RegionDwellNoHint {
                region_id: 7,
                region_tag: "Castle.ThroneRoom".into(),
                dwell_secs: 6
            }]
        );
        assert!(w.evaluate(secs(t0, 60), [0.0; 3], &[], &inside).is_empty());

        // A hint shortly BEFORE the server notices the entry still counts.
        let mut hinted = PlayerWatch::default();
        hinted.note_region_hint(7, t0);
        assert!(hinted
            .evaluate(secs(t0, 1), [0.0; 3], &[], &inside)
            .is_empty());
        assert!(hinted
            .evaluate(secs(t0, 30), [0.0; 3], &[], &inside)
            .is_empty());

        // Leaving and re-entering starts a fresh episode.
        assert!(w.evaluate(secs(t0, 70), [0.0; 3], &[], &[]).is_empty());
        assert!(w.evaluate(secs(t0, 72), [0.0; 3], &[], &inside).is_empty());
        assert_eq!(w.evaluate(secs(t0, 80), [0.0; 3], &[], &inside).len(), 1);
    }

    #[test]
    fn step_stalled_fires_once_per_step_and_resets_on_advance() {
        let t0 = Instant::now();
        let mut w = PlayerWatch::default();
        assert!(w.evaluate(t0, [0.0; 3], &[(706, 2411)], &[]).is_empty());
        assert!(w
            .evaluate(secs(t0, 299), [0.0; 3], &[(706, 2411)], &[])
            .is_empty());
        let fired = w.evaluate(secs(t0, 300), [0.0; 3], &[(706, 2411)], &[]);
        assert_eq!(
            fired,
            vec![Friction::StepStalled {
                mission_id: 706,
                step_id: 2411,
                age_secs: 300
            }]
        );
        assert!(w
            .evaluate(secs(t0, 900), [0.0; 3], &[(706, 2411)], &[])
            .is_empty());
        // Advancing restarts the clock for the new step.
        assert!(w
            .evaluate(secs(t0, 901), [0.0; 3], &[(706, 2412)], &[])
            .is_empty());
        assert!(w
            .evaluate(secs(t0, 1100), [0.0; 3], &[(706, 2412)], &[])
            .is_empty());
        assert_eq!(
            w.evaluate(secs(t0, 1201), [0.0; 3], &[(706, 2412)], &[])
                .len(),
            1
        );
    }

    /// The 2026-09-18 respawn bug: hints before death, none after, player
    /// still travelling.
    #[test]
    fn death_then_silence_needs_prior_hints_time_and_travel() {
        let t0 = Instant::now();
        let mut w = PlayerWatch::default();
        for r in 0..3 {
            w.note_region_hint(r, t0);
        }
        w.note_respawn(secs(t0, 10));
        // Time alone is not enough -- the player has not moved.
        assert!(w.evaluate(secs(t0, 200), [0.0; 3], &[], &[]).is_empty());
        // Walk 120 units in sub-teleport hops.
        let mut x = 0.0;
        let mut t = 200;
        let mut fired = Vec::new();
        while x < 120.0 {
            x += 20.0;
            t += 2;
            fired.extend(w.evaluate(secs(t0, t), [x, 0.0, 0.0], &[], &[]));
        }
        assert_eq!(fired.len(), 1, "fires exactly once: {fired:?}");
        assert!(matches!(
            fired[0],
            Friction::DeathThenSilence {
                hints_before_respawn: 3,
                ..
            }
        ));

        // A hint after respawn means the client is fine.
        let mut ok = PlayerWatch::default();
        for r in 0..3 {
            ok.note_region_hint(r, t0);
        }
        ok.note_respawn(secs(t0, 10));
        ok.note_region_hint(9, secs(t0, 20));
        ok.evaluate(secs(t0, 21), [0.0; 3], &[], &[]);
        assert!(ok
            .evaluate(secs(t0, 400), [40.0, 0.0, 0.0], &[], &[])
            .is_empty());
    }

    /// Dialog 2516 replaced by 5859 after 0.6 s.
    #[test]
    fn dialog_displaced_only_for_a_different_dialog_inside_the_window() {
        let t0 = Instant::now();
        let mut w = PlayerWatch::default();
        assert_eq!(w.note_dialog(2516, t0), None);
        assert_eq!(
            w.note_dialog(5859, t0 + Duration::from_millis(600)),
            Some(Friction::DialogDisplaced {
                dialog_id: 5859,
                replaced_dialog_id: 2516,
                ms_since_previous: 600
            })
        );
        assert_eq!(w.note_dialog(5859, t0 + Duration::from_millis(700)), None);
        assert_eq!(w.note_dialog(1, secs(t0, 10)), None);
    }

    #[test]
    fn point_in_polygon_handles_rotated_boxes() {
        // A diamond: its AABB contains (0.9, 0.9) but the polygon does not.
        let diamond = [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [-1.0, 0.0, 0.0],
        ];
        assert!(region_contains_xz(&diamond, 0.0, 0.0));
        assert!(region_contains_xz(&diamond, 0.4, 0.4));
        assert!(!region_contains_xz(&diamond, 0.9, 0.9));
        assert!(!region_contains_xz(&diamond[..2], 0.0, 0.0));
    }
}

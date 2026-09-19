//! Per-player watch half of the stuck-player detectors: the signals that need
//! state across time (`step_stalled`, `region_dwell_no_hint`,
//! `death_then_silence`) or fire at a gameplay event (`dialog_displaced`,
//! `objective_never_completed`, `escort_leader_teleported`). The episode
//! counters and the module-level rationale are in [`super::playtest_friction`].

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

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

// The XZ containment test moved to `spawner::regions`, beside the loader
// that builds `points` and beside the security gate that H06 layers on top
// of it (`is_point_in_region`). Re-exported rather than relocated at the
// call sites so `playtest_friction::region_contains_xz` — and this file's
// own polygon tests — keep working unchanged.
pub(crate) use crate::cell::spawner::region_contains_xz;

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

/// The previous dialog shown to this player and how long ago, if any.
pub(crate) fn last_dialog(entity_id: u32) -> Option<(i32, u64)> {
    let now = Instant::now();
    with_watch(entity_id, |w| {
        w.last_dialog
            .map(|(id, at)| (id, now.saturating_duration_since(at).as_millis() as u64))
    })
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

/// A player is about to be teleported (ring transport, GM travel, respawn).
/// Any NPC following them is left behind: follow re-paths from where it stands
/// and has no notion of the leader having changed floors or rooms.
pub(crate) fn leader_teleported(
    space_mgr: &crate::cell::space_manager::SpaceManager,
    leader_id: u32,
) {
    let Some(leader_pos) = space_mgr.get_entity(leader_id).map(|e| e.position) else {
        return;
    };
    for npc_id in space_mgr.all_npc_entity_ids() {
        let Some(npc) = space_mgr.get_entity(npc_id) else {
            continue;
        };
        if npc.follow_target_id != Some(leader_id) {
            continue;
        }
        tracing::warn!(
            target: "playtest.friction",
            signal = "escort_leader_teleported",
            reason = "leader_teleported",
            npc_id,
            npc_name = npc.npc_name.as_deref().unwrap_or(""),
            tag = npc.tag.as_deref().unwrap_or(""),
            target_id = leader_id,
            ai_state = ?npc.ai_state,
            nav_path_len = npc.nav_path.len(),
            dist_before_teleport = npc.position.distance_to(&leader_pos),
            "friction: a followed player is teleporting -- the escort stays where it is and must path to the new location on foot"
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

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
pub const WATCH_EVAL_INTERVAL: Duration = Duration::from_secs(2);
/// A mission step this old, on a player who is still sending movement, is
/// reported once.
pub const STEP_STALL_AFTER: Duration = Duration::from_secs(300);
/// Server-side containment must hold this long with no client hint.
pub const REGION_DWELL_AFTER: Duration = Duration::from_secs(6);
/// A hint this recent counts for a region the server only now sees entered
/// (the client detects the edge before our 2 s evaluation does).
pub const REGION_HINT_GRACE: Duration = Duration::from_secs(30);
/// A second dialog this soon after the first replaces it before it can be read.
pub const DIALOG_DISPLACED_WITHIN: Duration = Duration::from_secs(3);
/// Post-respawn silence: no region hint after this long...
pub const RESPAWN_SILENCE_AFTER: Duration = Duration::from_secs(120);
/// ...and this much travel, for a client that was hinting before it died.
pub const RESPAWN_SILENCE_MIN_TRAVEL: f32 = 100.0;
pub const RESPAWN_SILENCE_MIN_PRIOR_HINTS: u32 = 3;
/// Position deltas above this between two evaluations are teleports, not travel.
const TELEPORT_JUMP: f32 = 50.0;

/// One detected condition. Pure data so the detector logic is testable
/// without a tracing subscriber.
#[derive(Debug, Clone, PartialEq)]
pub enum Friction {
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
    /// The player closed or answered `last_dialog` (its `dialogButtonChoice`
    /// was accepted), so whatever is displayed next did not displace it.
    last_dialog_answered: bool,
    /// The player's character name, refreshed on each evaluation, so the
    /// signals raised from gameplay hooks that hold no `SpaceManager` can
    /// still name the player (Rule 6). Log-only.
    entity_name: Option<&'static str>,
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

    /// The player's choice for `dialog_id` was accepted. Only the dialog
    /// currently on record counts: an older one answered late (the client's
    /// eviction close, F13) says nothing about the one that replaced it.
    pub(crate) fn note_dialog_answered(&mut self, dialog_id: i32) {
        if self.last_dialog.is_some_and(|(id, _)| id == dialog_id) {
            self.last_dialog_answered = true;
        }
    }

    pub(crate) fn note_dialog(&mut self, dialog_id: i32, now: Instant) -> Option<Friction> {
        let out = match self.last_dialog {
            Some((prev, at))
                if prev != dialog_id
                    && !self.last_dialog_answered
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
        self.last_dialog_answered = false;
        out
    }

    /// Re-evaluate the time-based signals. `missions` is
    /// `(mission_id, current_step_id)` for every active, non-hidden mission;
    /// `regions_inside` is every region the client's own hit test puts `pos`
    /// inside ([`regions_client_should_hint`]).
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
pub use crate::cell::spawner::region_contains_xz;

static WATCHES: LazyLock<Mutex<HashMap<u32, PlayerWatch>>> = LazyLock::new(Mutex::default);

fn with_watch<R>(entity_id: u32, f: impl FnOnce(&mut PlayerWatch) -> R) -> R {
    let mut guard = WATCHES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(guard.entry(entity_id).or_default())
}

/// The name `player_tick` last recorded for `entity_id`, without creating a
/// watch for an entity that has none.
fn watched_name(entity_id: u32) -> Option<&'static str> {
    WATCHES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&entity_id)
        .and_then(|w| w.entity_name)
}

fn emit(entity_id: u32, entity_name: Option<&'static str>, f: &Friction) {
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
            entity_name,
            mission_id,
            mission_name = cimmeria_names::book().mission(*mission_id),
            step_id,
            step_name = cimmeria_names::book().mission_step(*step_id),
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
            entity_name,
            region_id, // nt:id-only runtime region id; region_tag on this line names it
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
            entity_name,
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
            entity_name,
            dialog_id,
            dialog_name = cimmeria_names::book().dialog(*dialog_id),
            replaced_dialog_id,
            replaced_dialog_name = cimmeria_names::book().dialog(*replaced_dialog_id),
            ms_since_previous,
            "friction: a dialog was replaced before the player could read it"
        ),
    }
}

/// Called on every accepted player movement packet; self-throttles to
/// [`WATCH_EVAL_INTERVAL`] per player.
pub fn player_tick(
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
    let regions = regions_client_should_hint(&space_mgr.regions_for_world(&world), pos);
    let entity_name = crate::cell::space_manager::EntityNames::of(e).entity_name;
    let fired = with_watch(entity_id, |w| {
        w.entity_name = entity_name;
        w.evaluate(now, pos, &missions, &regions)
    });
    for f in &fired {
        emit(entity_id, entity_name, f);
    }
}

/// The regions `region_dwell_no_hint` may complain about at `pos`: the ones
/// registered with the client (`REGION_FLAG_CLIENT_HINTED`) whose volume the
/// client's own hit test puts `pos` inside. An XZ-only test here reported
/// Castle_Cellblock Region6/Region12 for players on the floor above them,
/// where the client correctly stays silent (2026-09-26 colo logs).
pub(crate) fn regions_client_should_hint(
    regions: &[&crate::cell::space_manager::RegionData],
    pos: [f32; 3],
) -> Vec<(u32, String)> {
    regions
        .iter()
        .filter(|r| r.flags & crate::cell::space_manager::REGION_FLAG_CLIENT_HINTED != 0)
        .filter(|r| {
            crate::cell::spawner::client_would_hint_region(&r.points, r.height, r.radius, pos)
        })
        .map(|r| (r.runtime_id, r.tag.clone()))
        .collect()
}

/// The client reported a region edge (`triggerClientHintedGenericRegion`).
pub fn region_hint(entity_id: u32, region_id: u32) {
    with_watch(entity_id, |w| w.note_region_hint(region_id, Instant::now()));
}

/// The player respawned (`callForAid`).
pub fn respawned(entity_id: u32) {
    with_watch(entity_id, |w| w.note_respawn(Instant::now()));
}

/// The previous dialog shown to this player and how long ago, if any.
pub fn last_dialog(entity_id: u32) -> Option<(i32, u64)> {
    let now = Instant::now();
    with_watch(entity_id, |w| {
        w.last_dialog
            .map(|(id, at)| (id, now.saturating_duration_since(at).as_millis() as u64))
    })
}

/// A dialog is about to be displayed to the player.
pub fn dialog_shown(entity_id: u32, dialog_id: i32) {
    let fired = with_watch(entity_id, |w| {
        w.note_dialog(dialog_id, Instant::now())
            .map(|f| (f, w.entity_name))
    });
    if let Some((f, entity_name)) = fired {
        emit(entity_id, entity_name, &f);
    }
}

/// The player closed or answered `dialog_id` (its choice passed the #479 gate).
pub fn dialog_answered(entity_id: u32, dialog_id: i32) {
    with_watch(entity_id, |w| w.note_dialog_answered(dialog_id));
}

/// A mission is being completed by a chain while objectives are still open.
/// `open` holds only objectives some chain completes on its own; turn-in
/// objectives that only `CompleteMission` closes are filtered out by the
/// caller (`content::executor::mission::report_objectives_left_open`).
pub fn objectives_never_completed(entity_id: u32, mission_id: i32, open: &[(i32, bool)]) {
    if open.is_empty() {
        return;
    }
    let entity_name = watched_name(entity_id);
    for &(objective_id, optional) in open {
        tracing::warn!(
            target: "playtest.friction",
            signal = "objective_never_completed",
            reason = "objective_open_at_mission_complete",
            entity_id,
            entity_name,
            mission_id,
            mission_name = cimmeria_names::book().mission(mission_id),
            objective_id,
            objective_name = cimmeria_names::book().mission_objective(objective_id),
            optional,
            "friction: mission completed with an objective the player never completed -- its trigger may be unreachable"
        );
    }
}

/// A player is about to be teleported (ring transport, GM travel, respawn).
/// Any NPC following them is left behind: follow re-paths from where it stands
/// and has no notion of the leader having changed floors or rooms.
pub fn leader_teleported(space_mgr: &crate::cell::space_manager::SpaceManager, leader_id: u32) {
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
        let names = crate::cell::space_manager::EntityNames::of(npc);
        tracing::warn!(
            target: "playtest.friction",
            signal = "escort_leader_teleported",
            reason = "leader_teleported",
            npc_id,
            npc_name = names.entity_name,
            template_id = names.template_id,
            template_name = names.template_name,
            tag = npc.tag.as_deref(),
            target_id = leader_id,
            target_name = space_mgr.entity_label(leader_id),
            ai_state = ?npc.ai_state(),
            nav_path_len = npc.nav_path.len(),
            dist_before_teleport = npc.position.distance_to(&leader_pos),
            "friction: a followed player is teleporting -- the escort stays where it is and must path to the new location on foot"
        );
    }
}

/// Drop all per-entity state (entity ids are recycled).
pub fn forget(entity_id: u32) {
    WATCHES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&entity_id);
}

#[cfg(test)]
mod tests;

//! NPC AI detectors: rows that name a stuck, floating or stale NPC while it
//! is happening (NPC AI restoration packet NA02, audit gap T5).
//!
//! **Nothing here changes a decision.** Every function reads state the AI
//! and movement ticks already produced and reports on it; the detectors are
//! the before-picture the behaviour packets (NA1x) are measured against, so
//! they must fire on today's bugs.
//!
//! Conventions (see `docs/analysis/npc-ai-restoration/telemetry.md`):
//!
//! - every row carries `npc_id, tag, template_id, world, space_id`
//!   ([`NpcIdent`]);
//! - a player-visible bad state is a WARN, throttled per `(npc_id, kind)`
//!   with `suppressed = N` ([`NpcDetectors::admit_warn`]), beside an
//!   **unthrottled** counter labelled only by `world` and small enums;
//! - every per-entity slot is released by [`NpcDetectors::forget`], which
//!   `SpaceManager::destroy_entity` **and** `destroy_space` call (the second
//!   path is the one PR #726 found leaking).
//!
//! Layout:
//!
//! - [`movement`] — per movement tick (100 ms): `stale_velocity` (which
//!   also carries the telemetry plan's `animating_without_path` shape, see
//!   that module), `ground_deviation`, and the moved-since-last flag for
//!   `wire.out.avatar_update`.
//! - [`sweep`] — per AI tick (2 s): the idle-unticked gauge, and for
//!   ticked NPCs `npc_off_mesh` and `stuck`.
//! - [`leash`] — `npc_ai.leash` enter / snap_fallback / loop / damage_ignored.
//! - [`aggro_scan`] — the Idle scan's `candidate_rejected` / `no_candidates`.
//! - [`threat`] — `threat event=cleared_without_exit`.
//! - [`idle_parked`] — `npc_ai.idle_parked`.
//! - [`los`] — `npc_ai.los`, the sampled line-of-sight evidence row.
//! - [`spawn`] — `spawner.npc_behaviour event=spawn_off_mesh`.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cimmeria_common::Vector3;

use crate::cell::space_manager::{LogThrottle, SpaceManager};

pub(in crate::cell) mod aggro_scan;
pub(in crate::cell) mod idle_parked;
pub(in crate::cell) mod leash;
pub(in crate::cell) mod los;
pub(in crate::cell) mod movement;
pub(in crate::cell) mod spawn;
pub(in crate::cell) mod sweep;
pub(in crate::cell) mod threat;

#[cfg(test)]
mod tests;

/// The common identity every NPC row carries. Owned so a caller can resolve
/// it before taking `&mut` on the manager.
#[derive(Debug, Clone)]
pub(in crate::cell) struct NpcIdent {
    pub tag: String,
    pub template_id: i32,
    pub world: String,
    pub space_id: u32,
}

impl NpcIdent {
    pub(in crate::cell) fn of(space_mgr: &SpaceManager, npc_id: u32) -> Option<Self> {
        let e = space_mgr.get_entity(npc_id)?;
        Some(Self {
            tag: e.tag.clone().unwrap_or_default(),
            template_id: e.template_id.unwrap_or(0),
            world: super::world_label(space_mgr, npc_id),
            space_id: e.space_id.0 as u32,
        })
    }
}

/// Who last put this NPC where it is, for `npc_off_mesh`'s
/// `last_move_source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell) enum MoveSource {
    /// Stepping along a navmesh route.
    Path,
    /// Stepping toward a raw destination after a routing failure.
    Fallback,
    /// The instant leash snap to spawn.
    Leash,
    /// The min-range backup waypoint.
    Backup,
    /// A content-engine move.
    Content,
    /// Where it was spawned.
    Spawn,
}

impl MoveSource {
    pub(in crate::cell) fn label(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Fallback => "fallback",
            Self::Leash => "leash",
            Self::Backup => "backup",
            Self::Content => "content",
            Self::Spawn => "spawn",
        }
    }
}

/// Per-NPC movement-tick tracking.
#[derive(Debug, Default)]
pub(super) struct MoveTrack {
    last_pos: Option<Vector3>,
    /// Consecutive movement ticks with a non-zero velocity and no
    /// displacement.
    still_ticks: u32,
    /// Whether the position changed on the most recent movement tick.
    moved_last_tick: bool,
    /// Inside a `ground_deviation` episode (the last checked step was off
    /// the floor), so the counter counts episodes, not steps.
    off_ground: bool,
    /// What installed the NPC's current movement.
    source: Option<MoveSource>,
}

/// Per-NPC AI-tick tracking.
#[derive(Debug, Default)]
pub(super) struct AiTrack {
    /// `npc_to_target` on the last few chasing ticks, oldest first.
    chase_history: VecDeque<f32>,
    /// When this NPC entered Leashing, oldest first, pruned to the window.
    leash_times: VecDeque<Instant>,
}

/// All NPC-detector state. One struct so every teardown path makes one call.
#[derive(Debug, Default)]
pub(in crate::cell) struct NpcDetectors {
    /// Per `(npc, kind)` WARN windows.
    warn_log: LogThrottle,
    /// Per `(npc, kind)` DEBUG sampling windows.
    sample_log: LogThrottle,
    /// Per `(a, b, kind)` windows for rows about a pair of entities.
    pair_log: PairThrottle,
    /// `npc_ai.los` sampling. Behind a mutex because line of sight is a
    /// `&SpaceManager` query and the sample must still be rate-limited.
    los_log: Mutex<PairThrottle>,
    movement: HashMap<u32, MoveTrack>,
    ai: HashMap<u32, AiTrack>,
    /// Spawn ids already reported by `spawn_off_mesh` (once per spawn id,
    /// across respawns). Bounded by the seeded spawn count.
    spawn_warned: HashSet<i32>,
    /// Last `npc_ai_idle_unticked` value reported per world, so the
    /// up/down counter can be driven by deltas.
    idle_unticked_reported: HashMap<String, i64>,
    /// `npc_ai.idle event=unticked` sampling, per world: last emit and
    /// the rows skipped since. Released per world by `destroy_space`.
    idle_summary_log: HashMap<String, (Instant, u32)>,
    /// Set once the cover service has loaded, so a space created before it
    /// does not report "no cover" from an empty index.
    pub(in crate::cell) cover_load_done: bool,
}

impl NpcDetectors {
    /// Release every per-entity slot. Called from
    /// `SpaceManager::destroy_entity` and `destroy_space`.
    pub(in crate::cell) fn forget(&mut self, entity_id: u32) {
        self.warn_log.forget(entity_id);
        self.sample_log.forget(entity_id);
        self.pair_log.forget(entity_id);
        self.los_log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .forget(entity_id);
        self.movement.remove(&entity_id);
        self.ai.remove(&entity_id);
    }

    /// WARN gate: `Some(suppressed)` to emit, `None` to count silently.
    pub(in crate::cell) fn admit_warn(
        &mut self,
        npc_id: u32,
        kind: &'static str,
        now: Instant,
        interval: Duration,
    ) -> Option<u32> {
        self.warn_log.admit(npc_id, kind, now, interval)
    }

    /// DEBUG sampling gate, same shape as [`Self::admit_warn`].
    pub(in crate::cell) fn admit_sample(
        &mut self,
        npc_id: u32,
        kind: &'static str,
        now: Instant,
        interval: Duration,
    ) -> Option<u32> {
        self.sample_log.admit(npc_id, kind, now, interval)
    }

    /// Sampling gate for the per-world `npc_ai.idle` summary, same shape
    /// as [`Self::admit_sample`].
    pub(in crate::cell) fn admit_world_summary(
        &mut self,
        world: &str,
        now: Instant,
        interval: Duration,
    ) -> Option<u32> {
        match self.idle_summary_log.get_mut(world) {
            None => {
                self.idle_summary_log.insert(world.to_string(), (now, 0));
                Some(0)
            }
            Some((last, suppressed)) => {
                if now.saturating_duration_since(*last) >= interval {
                    let n = *suppressed;
                    *last = now;
                    *suppressed = 0;
                    Some(n)
                } else {
                    *suppressed = suppressed.saturating_add(1);
                    None
                }
            }
        }
    }

    /// Release per-world sampling state when a space of `world` is torn
    /// down. The gauge baseline (`idle_unticked_reported`) is kept: it is
    /// bounded by the world count and must survive to drive the gauge back
    /// to zero.
    pub(in crate::cell) fn forget_world(&mut self, world: &str) {
        self.idle_summary_log.remove(world);
    }

    #[cfg(test)]
    pub(in crate::cell) fn tracks_world_summary(&self, world: &str) -> bool {
        self.idle_summary_log.contains_key(world)
    }

    /// Record what installed an NPC's current movement.
    pub(in crate::cell) fn note_move_source(&mut self, npc_id: u32, source: MoveSource) {
        self.movement.entry(npc_id).or_default().source = Some(source);
    }

    pub(in crate::cell) fn move_source(&self, npc_id: u32) -> Option<MoveSource> {
        self.movement.get(&npc_id).and_then(|m| m.source)
    }

    /// Whether the NPC's position changed on the most recent movement tick.
    /// `None` until the detector pass has seen it once (and for players).
    pub(in crate::cell) fn moved_last_tick(&self, npc_id: u32) -> Option<bool> {
        self.movement.get(&npc_id).map(|m| m.moved_last_tick)
    }

    /// Test-only: slots held for `entity_id`, per map, for the teardown
    /// leak guard. Adding a per-entity map means adding it here.
    #[cfg(test)]
    pub(in crate::cell) fn slots_for(&self, entity_id: u32) -> [(&'static str, usize); 6] {
        [
            (
                "movement",
                usize::from(self.movement.contains_key(&entity_id)),
            ),
            ("ai", usize::from(self.ai.contains_key(&entity_id))),
            ("pair_log", self.pair_log.tracked_for(entity_id)),
            (
                "los_log",
                self.los_log
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .tracked_for(entity_id),
            ),
            ("warn_log", self.warn_log.tracked_for(entity_id)),
            ("sample_log", self.sample_log.tracked_for(entity_id)),
        ]
    }

    /// Test-only: fill every per-entity map for `npc_id` (paired with
    /// `other` in the pair throttles), so a teardown guard cannot pass on an
    /// empty map.
    #[cfg(test)]
    pub(in crate::cell) fn fill_all_for_test(&mut self, npc_id: u32, other: u32, now: Instant) {
        let d = Duration::from_secs(1);
        self.movement.entry(npc_id).or_default().last_pos = Some(Vector3::new(0.0, 0.0, 0.0));
        self.ai
            .entry(npc_id)
            .or_default()
            .chase_history
            .push_back(1.0);
        self.pair_log.admit(npc_id, other, "test", now, d);
        self.los_log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .admit(npc_id, other, "test", now, d);
        self.warn_log.admit(npc_id, "test", now, d);
        self.sample_log.admit(npc_id, "test", now, d);
    }
}

/// [`LogThrottle`] for rows about a *pair* of entities (NPC and player).
/// Same contract: first occurrence emits, later ones inside the window are
/// counted and reported as `suppressed` on the next row that gets through.
#[derive(Debug, Default)]
pub(in crate::cell) struct PairThrottle {
    entries: HashMap<(u32, u32, &'static str), (Instant, u32)>,
}

impl PairThrottle {
    pub(in crate::cell) fn admit(
        &mut self,
        a: u32,
        b: u32,
        kind: &'static str,
        now: Instant,
        interval: Duration,
    ) -> Option<u32> {
        match self.entries.get_mut(&(a, b, kind)) {
            None => {
                self.entries.insert((a, b, kind), (now, 0));
                Some(0)
            }
            Some((last, suppressed)) => {
                if now.saturating_duration_since(*last) >= interval {
                    let n = *suppressed;
                    *last = now;
                    *suppressed = 0;
                    Some(n)
                } else {
                    *suppressed = suppressed.saturating_add(1);
                    None
                }
            }
        }
    }

    /// Drop every pair that names `entity_id` on either side.
    pub(in crate::cell) fn forget(&mut self, entity_id: u32) {
        self.entries
            .retain(|(a, b, _), _| *a != entity_id && *b != entity_id);
    }

    #[cfg(test)]
    fn tracked_for(&self, entity_id: u32) -> usize {
        self.entries
            .keys()
            .filter(|(a, b, _)| *a == entity_id || *b == entity_id)
            .count()
    }
}
